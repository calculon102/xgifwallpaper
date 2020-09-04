mod placement;
mod screen_info;
mod shm;
mod xatoms;

use clap::{value_t, App, Arg, ArgMatches};

use pix::rgb::Rgba8;

use std::ffi::{c_void, CString};
use std::fs::File;
use std::io::BufReader;
use std::os::raw::{c_char, c_int, c_uint, c_ulong};
use std::ptr;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::{thread, time};

use x11::xlib::*;

use placement::*;
use screen_info::*;
use shm::*;
use xatoms::*;

// TODO v0.2: Refactor frame-preparation and animation-loop out of prototyping-state
// TODO v0.2: placement as argument
// TODO v0.3: Multi-root handling

const ARG_COLOR: &str = "COLOR";
const ARG_DELAY: &str = "DELAY";
const ARG_PATH_TO_GIF: &str = "PATH_TO_GIF";
const ARG_VERBOSE: &str = "VERBOSE";

const EXIT_XSHM_UNSUPPORTED: i32 = 1;
const EXIT_UNKOWN_COLOR: i32 = 2;
const EXIT_INVALID_DELAY: i32 = 3;

struct Frame {
    delay: time::Duration,
    placements: Vec<ImagePlacement>,
    raster: Rc<Vec<c_char>>,
    ximage: Box<XImage>,
    xshminfo: Box<x11::xshm::XShmSegmentInfo>, // Must exist as long ximage is used
}

pub struct XContext {
    display: *mut x11::xlib::Display,
    screen: c_int,
    root: c_ulong,
    gc: GC,
    pixmap: Pixmap,
}

pub struct Options<'a> {
    background_color: &'a str,
    default_delay: u16,
    path_to_gif: &'a str,
    verbose: bool,
}

fn main() {
    if !is_xshm_available() {
        eprintln!("The X server in use does not support the shared memory extension (xshm).");
        std::process::exit(EXIT_XSHM_UNSUPPORTED);
    }

    let args = init_args();
    let options = parse_args(&args);

    let running = Arc::new(AtomicBool::new(true));

    init_sigint_handler(options.clone(), running.clone());

    // Pixmap of struct must be set later, is mut therefore
    let mut xcontext = create_xcontext();

    let color = parse_color(&xcontext, options.background_color);

    let mut frames = prepare_frames(xcontext.display, &color, options.clone(), running.clone());

    xcontext.pixmap = prepare_pixmap(&xcontext, &color);

    clear_background(&xcontext, options.clone());

    do_animation(&xcontext, &mut frames, running.clone());

    clean_up(xcontext, &mut frames);
}

fn parse_args<'a>(args: &'a ArgMatches<'a>) -> Arc<Options<'a>> {
    let delay = value_t!(args, ARG_DELAY, u16).unwrap_or_else(|_e| {
        eprintln!(
            "Use a value between {} and {} as default-delay.",
            u16::MIN,
            u16::MAX
        );
        std::process::exit(EXIT_INVALID_DELAY)
    });

    Arc::new(Options {
        background_color: args.value_of(ARG_COLOR).unwrap(),
        default_delay: delay,
        path_to_gif: args.value_of(ARG_PATH_TO_GIF).unwrap(),
        verbose: args.is_present(ARG_VERBOSE),
    })
}

fn init_args<'a>() -> ArgMatches<'a> {
    App::new("xgifwallpaper")
        .version("0.1.2")
        .author("Frank Großgasteiger <frank@grossgasteiger.de>")
        .about("Animates a GIF as wallpaper in your X-session")
        .arg(
            Arg::with_name(ARG_COLOR)
                .short("b")
                .long("background-color")
                .takes_value(true)
                .value_name("X11-color")
                .default_value("#000000")
                .help("X11 compilant color-name to paint background."),
        )
        .arg(
            Arg::with_name(ARG_DELAY)
                .short("d")
                .long("default-delay")
                .takes_value(true)
                .value_name("default-delay")
                .default_value("10")
                .help("Delay in centiseconds between frames, if unspecified in GIF."),
        )
        .arg(Arg::with_name(ARG_VERBOSE).short("v").help("Verbose mode"))
        .arg(
            Arg::with_name(ARG_PATH_TO_GIF)
                .help("Path to GIF-file")
                .required(true)
                .index(1),
        )
        .get_matches()
}

fn init_sigint_handler<'a>(options: Arc<Options<'a>>, running: Arc<AtomicBool>) {
    let verbose = options.verbose;

    ctrlc::set_handler(move || {
        running.store(false, Ordering::SeqCst);

        if verbose {
            println!("SIGINT received");
        }
    })
    .expect("Error setting Ctrl-C handler");
}

fn create_xcontext() -> Box<XContext> {
    let display = unsafe { XOpenDisplay(ptr::null()) };
    let screen = unsafe { XDefaultScreen(display) };
    let gc = unsafe { XDefaultGC(display, screen) };
    let root = unsafe { XRootWindow(display, screen) };

    Box::new(XContext {
        display: display,
        screen: screen,
        gc: gc,
        root: root,
        pixmap: 0,
    })
}

fn parse_color(xcontext: &XContext, color_str: &str) -> Box<XColor> {
    let mut xcolor: XColor = XColor {
        pixel: 0,
        red: 0,
        green: 0,
        blue: 0,
        flags: 0,
        pad: 0,
    };

    let xcolor_ptr: *mut XColor = &mut xcolor;

    let cmap = unsafe { XDefaultColormap(xcontext.display, xcontext.screen) };
    let result = unsafe {
        XParseColor(
            xcontext.display,
            cmap,
            CString::new(color_str).unwrap().as_ptr(),
            xcolor_ptr,
        )
    };

    if result == 0 {
        unsafe { XCloseDisplay(xcontext.display) };

        eprintln!(
            "Unable to parse {} as X11-color. Try hex-color format: #RRGGBB.",
            color_str
        );
        std::process::exit(EXIT_UNKOWN_COLOR);
    }

    unsafe { XAllocColor(xcontext.display, cmap, xcolor_ptr) };

    Box::new(xcolor)
}

fn prepare_pixmap(xcontext: &Box<XContext>, color: &Box<XColor>) -> Pixmap {
    let dsp = xcontext.display;
    let scr = xcontext.screen;
    let gc = xcontext.gc;

    unsafe {
        let dsp_width = XDisplayWidth(dsp, scr) as c_uint;
        let dsp_height = XDisplayHeight(dsp, scr) as c_uint;
        let depth = XDefaultDepth(dsp, scr) as c_uint;

        let pixmap = XCreatePixmap(dsp, xcontext.root, dsp_width, dsp_height, depth);

        XSetForeground(dsp, gc, color.pixel);
        XSetBackground(dsp, gc, color.pixel);
        XSetFillStyle(dsp, gc, FillSolid);

        XDrawRectangle(dsp, pixmap, gc, 0, 0, dsp_width, dsp_height);
        XFillRectangle(dsp, pixmap, gc, 0, 0, dsp_width, dsp_height);

        pixmap
    }
}

fn prepare_frames(
    xdisplay: *mut Display,
    color: &Box<XColor>,
    options: Arc<Options>,
    running: Arc<AtomicBool>,
) -> Vec<Frame> {
    // Decode gif-frames into raster-steps
    // TODO Try using only low-level frames
    let steps = create_decoder(options.path_to_gif).into_steps();
    let methods = gather_disposal_methods(options.path_to_gif);

    let mut out: Vec<Frame> = Vec::new();
    let mut frame_index = 0;

    for step_option in steps {
        if !running.load(Ordering::SeqCst) {
            break;
        }

        let step = step_option.expect("Empty step in animation");
        let raster = step.raster();

        let width = raster.width();
        let height = raster.height();

        if options.verbose {
            println!(
                "Convert step {} to XImage, delay: {:?}, width: {}, height: {}, method: {:?}",
                frame_index,
                step.delay_time_cs(),
                width,
                height,
                methods[frame_index]
            );
        }

        // Create shared memory segment and image structure
        let xscreen = unsafe { XDefaultScreenOfDisplay(xdisplay) };
        let xvisual = unsafe { XDefaultVisualOfScreen(xscreen) };

        let image_byte_size = (width * height * 4) as usize;
        let mut xshminfo = create_xshm_sgmnt_inf(image_byte_size).unwrap();
        let ximage =
            create_xshm_image(xdisplay, xvisual, &mut xshminfo, width, height, 24).unwrap();

        let is_rgb = unsafe { (*ximage).byte_order == x11::xlib::MSBFirst };
        let rgba_indices = if is_rgb {
            [0, 1, 2, 3] // RGBA
        } else {
            [2, 1, 0, 3] // BGRA
        };

        // Write frame image into owned byte-vector
        let i8_slice = unsafe { &*(raster.as_u8_slice() as *const [u8] as *const [i8]) };
        let mut data: Vec<i8> = Vec::with_capacity((width * height * 4) as usize);

        let s = 4;

        let background_rgba = [
            (color.red / 256) as i8,
            (color.green / 256) as i8,
            (color.blue / 256) as i8,
            -127 as i8,
        ];

        // Get previous frame as raster or plain color pane, if non-existing
        let prev_raster: Rc<Vec<i8>> = {
            if out.len() > 0 {
                out.last().unwrap().raster.clone()
            } else {
                let capacity: usize = raster.width() as usize
                    * raster.height() as usize
                    * std::mem::size_of::<Rgba8>();

                let mut solid_color: Vec<i8> = Vec::with_capacity(capacity);
                let mut solid_color_index: usize = 0;

                while solid_color_index < capacity {
                    solid_color.push(background_rgba[rgba_indices[0]]);
                    solid_color.push(background_rgba[rgba_indices[1]]);
                    solid_color.push(background_rgba[rgba_indices[2]]);
                    solid_color.push(background_rgba[rgba_indices[3]]);

                    solid_color_index += s;
                }

                Rc::new(solid_color)
            }
        };

        let mut i = 0;
        while i < i8_slice.len() {
            let alpha = i8_slice[i + 3] as u8;

            if alpha == 255 {
                data.push(i8_slice[i + rgba_indices[0]]);
                data.push(i8_slice[i + rgba_indices[1]]);
                data.push(i8_slice[i + rgba_indices[2]]);
                data.push(i8_slice[i + rgba_indices[3]]);
            } else if methods[frame_index] == gift::block::DisposalMethod::Keep {
                data.push(prev_raster[i + 0]);
                data.push(prev_raster[i + 1]);
                data.push(prev_raster[i + 2]);
                data.push(prev_raster[i + 3]);
            } else {
                data.push(background_rgba[rgba_indices[0]]);
                data.push(background_rgba[rgba_indices[1]]);
                data.push(background_rgba[rgba_indices[2]]);
                data.push(alpha as i8);
            }
            i += s;
        }

        // Copy raw data into shared memory segment of XImage
        let data_ptr: Rc<Vec<i8>> = Rc::new(data);
        unsafe {
            libc::memcpy(
                xshminfo.shmaddr as *mut c_void,
                data_ptr.as_ptr() as *mut _,
                image_byte_size,
            );
            (*ximage).data = xshminfo.shmaddr;
            x11::xshm::XShmAttach(xdisplay, xshminfo.as_mut() as *mut _);
        };

        let data_size = unsafe { ((*ximage).bytes_per_line * (*ximage).height) as usize };

        assert_eq!(
            data_ptr.len(),
            data_size,
            "data-vector must be same length (is {}) as its anticipated capacity and size (is {})",
            data_ptr.len(),
            data_size
        );

        let mut delay = step.delay_time_cs().unwrap_or(options.default_delay);
        if delay <= 0 {
            delay = options.default_delay;
        }

        out.push(Frame {
            delay: time::Duration::from_millis((delay * 10) as u64),
            placements: Vec::new(),
            raster: data_ptr,
            ximage: unsafe { Box::new(*ximage) },
            xshminfo: xshminfo,
        });

        frame_index = frame_index + 1;
    }

    add_placements(&mut out);

    return out;
}

fn create_decoder(filename: &str) -> gift::Decoder<BufReader<File>> {
    gift::Decoder::new(File::open(filename).expect("Unable to read file"))
}

fn gather_disposal_methods(filename: &str) -> Vec<gift::block::DisposalMethod> {
    let mut methods: Vec<gift::block::DisposalMethod> = Vec::new();
    let frames = create_decoder(filename).into_frames();
    for frame in frames {
        if frame.is_ok() {
            let f = frame.unwrap();

            if f.graphic_control_ext.is_some() {
                methods.push(f.graphic_control_ext.unwrap().disposal_method());
            }

            continue;
        }

        methods.push(gift::block::DisposalMethod::NoAction);
    }

    methods
}

fn add_placements(frames: &mut Vec<Frame>) {
    let screen_info = get_screen_info();
    for i in 0..(frames.len()) {
        let image_width = frames[i].ximage.width;
        let image_height = frames[i].ximage.height;

        for screen in &screen_info.screens {
            frames[i].placements.push(get_image_placement(
                image_width,
                image_height,
                screen.clone(),
                ImagePlacementStrategy::CENTER,
            ));
        }
    }
}

/// Clear previous backgrounds on root
fn clear_background(xcontext: &Box<XContext>, options: Arc<Options>) {
    remove_root_pixmap_atoms(&xcontext, options.clone());

    unsafe {
        XClearWindow(xcontext.display, xcontext.root);
        XSync(xcontext.display, False);
    }
}

fn do_animation(xcontext: &Box<XContext>, frames: &mut Vec<Frame>, running: Arc<AtomicBool>) {
    let display = xcontext.display;
    let pixmap = xcontext.pixmap;
    let gc = xcontext.gc;
    let root = xcontext.root;

    let atom_root = get_root_pixmap_atom(display);
    let atom_eroot = get_eroot_pixmap_atom(display);

    while running.load(Ordering::SeqCst) {
        for i in 0..(frames.len()) {
            if !running.load(Ordering::SeqCst) {
                break;
            }

            for j in 0..(frames[i].placements.len()) {
                unsafe {
                    x11::xshm::XShmPutImage(
                        display,
                        pixmap,
                        gc,
                        frames[i].ximage.as_mut() as *mut _,
                        frames[i].placements[j].src_x,
                        frames[i].placements[j].src_y,
                        frames[i].placements[j].dest_x,
                        frames[i].placements[j].dest_y,
                        frames[i].placements[j].width as c_uint,
                        frames[i].placements[j].height as c_uint,
                        False,
                    );
                }
            }

            if !update_root_pixmap_atoms(display, root, &pixmap, atom_root, atom_eroot) {
                println!("set_root_atoms failed!");
            }

            unsafe {
                XClearWindow(display, root);
                XSetWindowBackgroundPixmap(display, root, pixmap);
                XSync(display, False);
            }
            thread::sleep(frames[i].delay);
        }
    }

    delete_atom(&xcontext, atom_root);
    delete_atom(&xcontext, atom_eroot);
}

fn clean_up(xcontext: Box<XContext>, frames: &mut Vec<Frame>) {
    // Clean up
    for i in 0..(frames.len()) {
        // Don't need to call XDestroy image - heap is freed by rust-guarantees. :)
        unsafe { x11::xshm::XShmDetach(xcontext.display, frames[i].xshminfo.as_mut() as *mut _) };
        destroy_xshm_sgmnt_inf(&mut frames[i].xshminfo);
    }

    unsafe {
        XFreePixmap(xcontext.display, xcontext.pixmap);
        XCloseDisplay(xcontext.display);
    }
}
