mod placement;
mod screen_info;
mod shm;
mod xatoms;

use clap::{App, Arg, ArgMatches};

use gift::decode::Steps;
use gift::Decoder;

use pix::rgb::Rgba8;

use std::ffi::CString;
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

// TODO v0.1: default-delay as argument
// TODO v0.2: Refactor frame-preparation and animation-loop out of prototyping-state
// TODO v0.2: placement as argument
// TODO v0.3: Multi-root handling
// TODO Bugfix: Clear-Background after each frame for transparent GIFs

const ARG_COLOR: &str = "COLOR";
const ARG_PATH_TO_GIF: &str = "PATH_TO_GIF";
const ARG_VERBOSE: &str = "VERBOSE";

const EXIT_XSHM_UNSUPPORTED: i32 = 1;
const EXIT_UNKOWN_COLOR: i32 = 2;

struct Frame {
    delay: time::Duration,
    placements: Vec<ImagePlacement>,
    raster: Rc<Vec<c_char>>,
    ximage: Box<XImage>,
    xshminfo: Box<x11::xshm::XShmSegmentInfo>, // Must exist as long ximage is used
}

pub struct Options<'a> {
    background_color: &'a str,
    path_to_gif: &'a str,
    verbose: bool,
}

fn load_gif(filename: &str) -> Steps<BufReader<File>> {
    let file_in = File::open(filename).expect("Could not load gif");

    let decoder = Decoder::new(file_in);

    return decoder.into_steps();
}

fn loop_animation(options: Arc<Options>, running: Arc<AtomicBool>, steps: Steps<BufReader<File>>) {
    unsafe {
        let display = XOpenDisplay(ptr::null());
        let screen = XDefaultScreen(display);

        let color = parse_color(display, screen, options.background_color);

        let mut frames = prepare_frames(display, steps, &color, options.clone(), running.clone());

        // Single root-loop
        // TODO multi-root implementation
        let gc = XDefaultGC(display, screen);
        let root = XRootWindow(display, screen);

        let pixmap = prepare_pixmap(display, screen, root, &gc, &color);

        remove_root_pixmap_atoms(display, root, pixmap, options.clone());
        XClearWindow(display, root);
        XSync(display, False);

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

        let atom_root = get_root_pixmap_atom(display);
        let atom_eroot = get_eroot_pixmap_atom(display);

        while running.load(Ordering::SeqCst) {
            for i in 0..(frames.len()) {
                if !running.load(Ordering::SeqCst) {
                    break;
                }

                for j in 0..(frames[i].placements.len()) {
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

                if !update_root_pixmap_atoms(display, root, &pixmap, atom_root, atom_eroot) {
                    println!("set_root_atoms failed!");
                }

                XClearWindow(display, root);
                XSetWindowBackgroundPixmap(display, root, pixmap);
                XSync(display, False);

                thread::sleep(frames[i].delay);
            }
        }

        // Clean up
        for i in 0..(frames.len()) {
            // Don't need to call XDestroy image - heap is freed by rust-guarantees. :)
            x11::xshm::XShmDetach(display, frames[i].xshminfo.as_mut() as *mut _);
            destroy_xshm_sgmnt_inf(&mut frames[i].xshminfo);
        }

        XFreePixmap(display, pixmap);
        XCloseDisplay(display);
    }
}

fn prepare_frames(
    xdisplay: *mut Display,
    frames: Steps<BufReader<File>>,
    color: &Box<XColor>,
    options: Arc<Options>,
    running: Arc<AtomicBool>,
) -> Vec<Frame> {
    let mut out: Vec<Frame> = Vec::new();
    let mut frame_count = 0;

    for step_option in frames {
        if !running.load(Ordering::SeqCst) {
            break;
        }

        let step = step_option.expect("Empty step in animation");
        let raster = step.raster();

        let width = raster.width();
        let height = raster.height();

        if options.verbose {
            frame_count = frame_count + 1;
            println!(
                "Convert step {} to XImage, delay: {:?}, width: {}, height: {}",
                frame_count,
                step.delay_time_cs(),
                width,
                height
            );
        }

        // Write frame image into owned byte-vector
        let i8_slice = unsafe { &*(raster.as_u8_slice() as *const [u8] as *const [i8]) };
        let mut data: Vec<i8> = Vec::with_capacity((width * height * 4) as usize);

        let s = 4;

        let prev_raster: Rc<Vec<i8>> = {
            if out.len() > 0 {
                out.last().unwrap().raster.clone()
            } else {
                let capacity: usize = raster.width() as usize
                    * raster.height() as usize
                    * std::mem::size_of::<Rgba8>();

                let mut solid_color: Vec<i8> = Vec::with_capacity(capacity);
                let mut solid_color_index: usize = 0;

                let red = (color.red / 256) as i8;
                let green = (color.green / 256) as i8;
                let blue = (color.blue / 256) as i8;

                while solid_color_index < capacity {
                    solid_color.push(blue);
                    solid_color.push(green);
                    solid_color.push(red);
                    solid_color.push(-127); // Alpha, weird to use i8 for rgb-values, here 255

                    solid_color_index += s;
                }

                Rc::new(solid_color)
            }
        };

        let mut i = 0;
        while i < i8_slice.len() {
            let alpha = i8_slice[i + 3] as u8;

            if alpha == 255 {
                data.push(i8_slice[i]);
                data.push(i8_slice[i + 1]);
                data.push(i8_slice[i + 2]);
                data.push(i8_slice[i + 3]);
            } else {
                data.push(prev_raster[i]);
                data.push(prev_raster[i + 1]);
                data.push(prev_raster[i + 2]);
                data.push(prev_raster[i + 3]);
            }
            i += s;
        }

        let data_ptr: Rc<Vec<i8>> = Rc::new(data);

        let xscreen = unsafe { XDefaultScreenOfDisplay(xdisplay) };
        let xvisual = unsafe { XDefaultVisualOfScreen(xscreen) };

        let mut xshminfo =
            create_xshm_sgmnt_inf(data_ptr.clone(), (width * height * 4) as usize).unwrap();
        let ximage =
            create_xshm_image(xdisplay, xvisual, &mut xshminfo, width, height, 24).unwrap();
        unsafe {
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

        let mut delay = step.delay_time_cs().unwrap_or(10);
        if delay <= 0 {
            delay = 10;
        }

        out.push(Frame {
            delay: time::Duration::from_millis((delay * 10) as u64),
            placements: Vec::new(),
            raster: data_ptr,
            ximage: unsafe { Box::new(*ximage) },
            xshminfo: xshminfo,
        });
    }

    return out;
}

fn parse_color(display: *mut Display, screen: c_int, color_str: &str) -> Box<XColor> {
    let mut xcolor: XColor = XColor {
        pixel: 0,
        red: 0,
        green: 0,
        blue: 0,
        flags: 0,
        pad: 0,
    };

    let xcolor_ptr: *mut XColor = &mut xcolor;

    let cmap = unsafe { XDefaultColormap(display, screen) };
    let result = unsafe {
        XParseColor(
            display,
            cmap,
            CString::new(color_str).unwrap().as_ptr(),
            xcolor_ptr,
        )
    };

    if result == 0 {
        unsafe { XCloseDisplay(display) };

        eprintln!(
            "Unable to parse {} as X11-color. Try hex-color format: #RRGGBB.",
            color_str
        );
        std::process::exit(EXIT_UNKOWN_COLOR);
    }

    unsafe { XAllocColor(display, cmap, xcolor_ptr) };

    Box::new(xcolor)
}

fn prepare_pixmap(
    display: *mut Display,
    screen: c_int,
    root: c_ulong,
    gc_ref: &GC,
    color: &Box<XColor>,
) -> Pixmap {
    unsafe {
        let display_width = XDisplayWidth(display, screen) as c_uint;
        let display_height = XDisplayHeight(display, screen) as c_uint;
        let depth = XDefaultDepth(display, screen) as c_uint;

        let pixmap = XCreatePixmap(display, root, display_width, display_height, depth);

        let gc = *gc_ref;
        XSetForeground(display, gc, color.pixel);
        XSetBackground(display, gc, color.pixel);
        XSetFillStyle(display, gc, FillSolid);

        XDrawRectangle(display, pixmap, gc, 0, 0, display_width, display_height);
        XFillRectangle(display, pixmap, gc, 0, 0, display_width, display_height);

        pixmap
    }
}

fn main() {
    if !is_xshm_available() {
        eprintln!("The X server in use does not support the shared memory extension (xshm).");
        std::process::exit(EXIT_XSHM_UNSUPPORTED);
    }

    let args = init_args();

    let options = Arc::new(Options {
        background_color: args.value_of(ARG_COLOR).unwrap(),
        path_to_gif: args.value_of(ARG_PATH_TO_GIF).unwrap(),
        verbose: args.is_present(ARG_VERBOSE),
    });
    let running = Arc::new(AtomicBool::new(true));

    init_sigint_handler(options.clone(), running.clone());

    let steps = load_gif(options.path_to_gif);

    // TODO Scale GIF-Frames accordingly to params (Center, Scale, Fill)
    loop_animation(options.clone(), running, steps);
}

fn init_args<'a>() -> ArgMatches<'a> {
    return App::new("xgifwallpaper")
        .version("0.1")
        .author("Frank Grossgasteiger <frank@grossgasteiger.de>")
        .about("Animates a GIF as background in your X-session")
        .arg(
            Arg::with_name(ARG_COLOR)
                .short("c")
                .long("color")
                .takes_value(true)
                .value_name("X11-color")
                .default_value("#000000")
                .help("X11 compilant color-name to paint background."),
        )
        .arg(Arg::with_name(ARG_VERBOSE).short("v").help("Verbose mode"))
        .arg(
            Arg::with_name(ARG_PATH_TO_GIF)
                .help("Path to GIF-file")
                .required(true)
                .index(1),
        )
        .get_matches();
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
