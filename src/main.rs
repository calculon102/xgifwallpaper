mod position;
mod screen_info;
mod shm;
mod xatoms;

use clap::{value_t, App, Arg, ArgMatches};

use pix::rgb::Rgba8;

use std::collections::HashMap;
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

use position::*;
use screen_info::*;
use shm::*;
use xatoms::*;

const ARG_COLOR: &str = "COLOR";
const ARG_DELAY: &str = "DELAY";
const ARG_PATH_TO_GIF: &str = "PATH_TO_GIF";
const ARG_SCALE: &str = "SCALE";
const ARG_VERBOSE: &str = "VERBOSE";

const EXIT_XSHM_UNSUPPORTED: i32 = 101;
const EXIT_UNKOWN_COLOR: i32 = 102;
const EXIT_INVALID_DELAY: i32 = 103;

macro_rules! log {
    ($is_verbose:ident, $message:expr) => {
        if $is_verbose.verbose {
            println!($message);
        }
    };

    ($is_verbose:ident, $message:expr, $($args:expr),*) => {
        if $is_verbose.verbose {
            println!($message $(,$args)*);
        }
    };
}

/// Screens to render wallpapers on, with needed resolution. And the pre-
/// rendered frames in a seperate map.
struct Wallpapers {
    screens: Vec<WallpaperOnScreen>,
    frames_by_resolution: HashMap<Resolution, Vec<Frame>>,
}

/// Resolution and placement of a wallpaper on a screen.
struct WallpaperOnScreen {
    placement: ImagePlacement,
    resolution: Resolution,
    _screen: screen_info::Screen, // TODO Check if useful at some time
}

/// Combines x-structs, raster- and metadata for a singe frame.
struct Frame {
    delay: time::Duration,
    raster: Rc<Vec<c_char>>,
    ximage: Box<XImage>,
    xshminfo: Box<x11::xshm::XShmSegmentInfo>, // Must exist as long ximage is used
}

/// Runtime options as given by the caller of this program.
pub struct Options<'a> {
    background_color: &'a str,
    default_delay: u16,
    path_to_gif: &'a str,
    scaling: Scaling,
    verbose: bool,
}

/// X11-specific control-data and references.
pub struct XContext {
    display: *mut x11::xlib::Display,
    screen: c_int,
    root: c_ulong,
    gc: GC,
    pixmap: Pixmap,
}

/// Application entry-point
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

    let mut wallpapers = render_wallpapers(&xcontext, &color, options.clone(), running.clone());

    xcontext.pixmap = prepare_pixmap(&xcontext, &color);

    clear_background(&xcontext, options.clone());

    do_animation(&xcontext, &mut wallpapers, options.clone(), running.clone());

    clean_up(xcontext, wallpapers, options.clone());
}

/// Declare command-line-arguments.
fn init_args<'a>() -> ArgMatches<'a> {
    App::new("xgifwallpaper")
        .version("0.2.0")
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
        .arg(
            Arg::with_name(ARG_SCALE)
                .short("s")
                .long("scale")
                .takes_value(true)
                .possible_values(&["NONE", "FILL", "MAX"])
                .default_value("NONE")
                .help("Scale GIF-frames, relative to available screen."),
        )
        .get_matches()
}

/// Parse arguments from command line.
fn parse_args<'a>(args: &'a ArgMatches<'a>) -> Arc<Options<'a>> {
    let delay = value_t!(args, ARG_DELAY, u16).unwrap_or_else(|_e| {
        eprintln!(
            "Use a value between {} and {} as default-delay.",
            u16::MIN,
            u16::MAX
        );
        std::process::exit(EXIT_INVALID_DELAY)
    });

    let scaling = match args.value_of(ARG_SCALE).unwrap() {
        "NONE" => Scaling::NONE,
        "FILL" => Scaling::FILL,
        "MAX" => Scaling::MAX,
        &_ => Scaling::NONE, // Cannot happen, due to guarantee of args
    };

    Arc::new(Options {
        background_color: args.value_of(ARG_COLOR).unwrap(),
        default_delay: delay,
        path_to_gif: args.value_of(ARG_PATH_TO_GIF).unwrap(),
        scaling,
        verbose: args.is_present(ARG_VERBOSE),
    })
}

/// Register handler for interrupt-signal.
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

/// Establish connection and context to the X-server
fn create_xcontext() -> Box<XContext> {
    let display = unsafe { XOpenDisplay(ptr::null()) };
    let screen = unsafe { XDefaultScreen(display) };
    let gc = unsafe { XDefaultGC(display, screen) };
    let root = unsafe { XRootWindow(display, screen) };

    Box::new(XContext {
        display,
        screen,
        gc,
        root,
        pixmap: 0,
    })
}

/// Parse string as X11-color.
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

/// Create and prepare the pixmap, where the wallpaper is drawn onto.
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

/// Pre-render wallpaper-frames for all needed resolutions, determined by
/// actual screens, options and image-data
fn render_wallpapers(
    xcontext: &Box<XContext>,
    background_color: &Box<XColor>,
    options: Arc<Options>,
    running: Arc<AtomicBool>,
) -> Wallpapers {
    // Decode gif-frames into raster-steps
    // TODO Try using only low-level frames
    // TODO Prevent double-encoding, by re-using iterator?
    let mut steps = create_decoder(options.path_to_gif).into_steps();
    let methods = gather_disposal_methods(options.path_to_gif);

    // Determine image-resolution
    let first_step = create_decoder(options.path_to_gif)
        .into_steps()
        .nth(0)
        .unwrap()
        .unwrap();
    let raster = first_step.raster();
    let image_resolution = Resolution {
        width: raster.width(),
        height: raster.height(),
    };

    // Build wallpapers by screen
    let screen_info = get_screen_info();

    let mut screens: Vec<WallpaperOnScreen> = Vec::new();
    let mut frames_by_resolution: HashMap<Resolution, Vec<Frame>> = HashMap::new();

    for screen in screen_info.screens {
        log!(options, "Prepare wallpaper for screen {:?}", screen);

        // Gather target-resolution and image-placement for particular screen
        let screen_resolution = Resolution {
            width: screen.width,
            height: screen.height,
        };

        let target_resolution =
            compute_target_resolution(&image_resolution, &screen_resolution, &options.scaling);

        let placement = get_image_placement(&target_resolution, &screen, Alignment::CENTER);

        log!(options, "placement: {:?}", placement);

        let wallpaper_on_screen = WallpaperOnScreen {
            placement: get_image_placement(&target_resolution, &screen, Alignment::CENTER),
            resolution: target_resolution.clone(),
            _screen: screen.clone(),
        };

        // If frames were not already rendered for given resolution, do so
        if !frames_by_resolution.contains_key(&target_resolution) {
            frames_by_resolution.insert(
                target_resolution,
                render_frames(
                    xcontext,
                    background_color,
                    &wallpaper_on_screen,
                    steps.by_ref(),
                    &methods,
                    options.clone(),
                    running.clone(),
                ),
            );
        } else {
            log!(
                options,
                "Reuse already rendered frames for {:?}",
                target_resolution
            );
        }

        screens.push(wallpaper_on_screen);
    }

    Wallpapers {
        screens,
        frames_by_resolution,
    }
}

/// Create GIF-decoder from file.
fn create_decoder(path_to_gif: &str) -> gift::Decoder<BufReader<File>> {
    gift::Decoder::new(File::open(path_to_gif).expect("Unable to read file"))
}

/// Parse GIF to gather the disposal-method for each frame.
fn gather_disposal_methods(path_to_gif: &str) -> Vec<gift::block::DisposalMethod> {
    let mut methods: Vec<gift::block::DisposalMethod> = Vec::new();
    let frames = create_decoder(path_to_gif).into_frames();
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

/// Render GIF-frames as bitmaps for a specific screen.
fn render_frames(
    xcontext: &Box<XContext>,
    color: &Box<XColor>,
    wallpaper_on_screen: &WallpaperOnScreen,
    steps: &mut gift::decode::Steps<BufReader<File>>,
    methods: &Vec<gift::block::DisposalMethod>,
    options: Arc<Options>,
    running: Arc<AtomicBool>,
) -> Vec<Frame> {
    let mut out: Vec<Frame> = Vec::new();
    let mut frame_index = 0;

    let xscreen = unsafe { XDefaultScreenOfDisplay(xcontext.display) };
    let xvisual = unsafe { XDefaultVisualOfScreen(xscreen) };

    // Convert rasters to frames
    for step_option in steps.by_ref() {
        if !running.load(Ordering::SeqCst) {
            break;
        }

        let step = step_option.expect("Empty step in animation");
        let raster = step.raster();

        let image_resolution = Resolution {
            width: raster.width(),
            height: raster.height(),
        };

        let target_resolution = wallpaper_on_screen.resolution.clone();

        log!(
            options,
            "Convert step {} (delay: {:?}, method: {:?}, width: {}, height: {}) to XImage (width: {}, height: {})",
            frame_index,
            step.delay_time_cs(),
            methods[frame_index],
            image_resolution.width,
            image_resolution.height,
            target_resolution.width,
            target_resolution.height
        );

        // Build target raster
        // TODO Better naming of vectors
        let step_data = resize_raster(&raster, &target_resolution, options.clone());

        let mut data: Vec<i8> =
            Vec::with_capacity((target_resolution.width * target_resolution.height * 4) as usize);

        // Create shared memory segment and image structure
        let image_byte_size = (target_resolution.width * target_resolution.height * 4) as usize;
        let mut xshminfo = create_xshm_sgmnt_inf(image_byte_size).unwrap();
        let ximage = create_xshm_image(
            xcontext.display,
            xvisual,
            &mut xshminfo,
            target_resolution.width as u32,
            target_resolution.height as u32,
            24,
        )
        .unwrap();

        let is_rgb = unsafe { (*ximage).byte_order == x11::xlib::MSBFirst };
        let rgba_indices = if is_rgb {
            [0, 1, 2, 3] // RGBA
        } else {
            [2, 1, 0, 3] // BGRA
        };

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

        let u8_slice = step_data.as_slice();
        let mut i = 0;

        while i < u8_slice.len() {
            let alpha = u8_slice[i + 3];

            if alpha == 255 {
                data.push(u8_slice[i + rgba_indices[0]] as i8);
                data.push(u8_slice[i + rgba_indices[1]] as i8);
                data.push(u8_slice[i + rgba_indices[2]] as i8);
                data.push(u8_slice[i + rgba_indices[3]] as i8);
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
            x11::xshm::XShmAttach(xcontext.display, xshminfo.as_mut() as *mut _);
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
            raster: data_ptr,
            ximage: unsafe { Box::new(*ximage) },
            xshminfo,
        });

        frame_index = frame_index + 1;
    }

    return out;
}

/// Resize given RGBA-raster to target-resolution.
fn resize_raster(
    raster: &pix::Raster<pix::rgb::SRgba8>,
    target_resolution: &Resolution,
    options: Arc<Options>,
) -> Vec<u8> {
    let src_w = raster.width() as usize;
    let src_h = raster.height() as usize;
    let dst_w = target_resolution.width as usize;
    let dst_h = target_resolution.height as usize;

    let must_resize = src_w != dst_w || src_h != dst_h;

    if must_resize {
        let resize_type = if (src_w * src_h) > (dst_w * dst_h) {
            resize::Type::Lanczos3
        } else {
            resize::Type::Mitchell
        };

        log!(
            options,
            "Resize raster from {}x{} to {}x{}",
            src_w,
            src_h,
            dst_w,
            dst_h
        );

        let sample_size = dst_w * dst_h * 4;

        let mut dst: Vec<u8> = Vec::with_capacity(sample_size);
        dst.resize(sample_size, 0);

        let mut resizer = resize::new(src_w, src_h, dst_w, dst_h, resize::Pixel::RGBA, resize_type);

        resizer.resize(raster.as_u8_slice().to_vec().as_ref(), &mut dst);

        dst
    } else {
        raster.as_u8_slice().to_vec()
    }
}

/// Clear previous backgrounds on root.
fn clear_background(xcontext: &Box<XContext>, options: Arc<Options>) {
    remove_root_pixmap_atoms(&xcontext, options.clone());

    unsafe {
        XClearWindow(xcontext.display, xcontext.root);
        XSync(xcontext.display, False);
    }
}

/// Loops the pre-renders wallpapers on each screen. Will only stop on
/// interrupt-signal.
fn do_animation(
    xcontext: &Box<XContext>,
    wallpapers: &mut Wallpapers,
    options: Arc<Options>,
    running: Arc<AtomicBool>,
) {
    log!(options, "Loop animation...");

    let display = xcontext.display;
    let pixmap = xcontext.pixmap;
    let gc = xcontext.gc;
    let root = xcontext.root;

    let atom_root = get_root_pixmap_atom(display);
    let atom_eroot = get_eroot_pixmap_atom(display);

    let mut i: usize = 0;
    let mut delay: std::time::Duration = std::time::Duration::new(0, 0);

    while running.load(Ordering::SeqCst) {
        if !running.load(Ordering::SeqCst) {
            break;
        }

        for screen in &wallpapers.screens {
            let frames = wallpapers
                .frames_by_resolution
                .get_mut(&screen.resolution)
                .unwrap();

            // The following assumptions only work, while there is a single GIF
            // to render. Different GIFs per screen would require a rewrite.

            // Assumption: All framesets have same length
            if frames.len() <= i {
                i = 0;
            }

            // Assumption: All frames with same index have same delay
            delay = frames[i].delay;

            //log!(options, "Put frame {} on screen {:?}", i, screen.placement);

            unsafe {
                x11::xshm::XShmPutImage(
                    display,
                    pixmap,
                    gc,
                    &mut *frames[i].ximage,
                    screen.placement.src_x,
                    screen.placement.src_y,
                    screen.placement.dest_x,
                    screen.placement.dest_y,
                    screen.placement.width as c_uint,
                    screen.placement.height as c_uint,
                    False,
                );
            }
        }

        i = i + 1;

        if !update_root_pixmap_atoms(display, root, &pixmap, atom_root, atom_eroot) {
            eprintln!("set_root_atoms failed!");
        }

        unsafe {
            XClearWindow(display, root);
            XSetWindowBackgroundPixmap(display, root, pixmap);
            XSync(display, False);
        }

        thread::sleep(delay);
    }

    log!(options, "Stop animation-loop");

    delete_atom(&xcontext, atom_root);
    delete_atom(&xcontext, atom_eroot);
}

/// Clears reference and (shared-)-memory.
fn clean_up(xcontext: Box<XContext>, mut wallpapers: Wallpapers, options: Arc<Options>) {
    log!(options, "Free images in shared memory");

    for frames in wallpapers.frames_by_resolution.values_mut() {
        for i in 0..(frames.len()) {
            // Don't need to call XDestroy image - heap is freed by rust-guarantees. :)
            unsafe {
                x11::xshm::XShmDetach(xcontext.display, frames[i].xshminfo.as_mut() as *mut _)
            };
            destroy_xshm_sgmnt_inf(&mut frames[i].xshminfo);
        }
    }

    unsafe {
        log!(options, "Free pixmap used for background");
        XFreePixmap(xcontext.display, xcontext.pixmap);

        log!(options, "Reset background to solid black and clear window");
        XSetWindowBackground(
            xcontext.display,
            xcontext.root,
            x11::xlib::XBlackPixel(xcontext.display, xcontext.screen),
        );
        XClearWindow(xcontext.display, xcontext.root);

        XCloseDisplay(xcontext.display);
    }
}
