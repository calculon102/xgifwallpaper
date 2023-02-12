#[macro_use]
pub mod macros;

pub mod options;
mod position;
pub mod screens;
mod shm;
mod xatoms;
pub mod xcontext;

use pix::rgb::Rgba8;

use std::collections::HashMap;
use std::ffi::c_void;
use std::fs::File;
use std::os::raw::{c_uchar, c_uint};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::{thread, time};

use x11::xlib::*;

use options::Options;
use position::*;
use screens::*;
use shm::*;
use xatoms::*;
use xcontext::XContext;

const EXIT_INVALID_FILE: i32 = 103;

const VERSION: &str = "0.3.2";

/// Screens to render wallpapers on, with needed resolution. And the pre-
/// rendered frames in a seperate map.
pub struct Wallpapers {
    screens: Vec<WallpaperOnScreen>,
    frames_by_resolution: HashMap<Resolution, Vec<Frame>>,
}

/// Resolution and placement of a wallpaper on a screen.
struct WallpaperOnScreen {
    placement: ImagePlacement,
    resolution: Resolution,
    _screen: screens::Screen, // TODO Check if useful at some time
}

/// Combines x-structs, raster- and metadata for a singe frame.
struct Frame {
    delay: time::Duration,
    raster: Rc<Vec<c_uchar>>,
    ximage: Box<XImage>,
    xshminfo: Box<x11::xshm::XShmSegmentInfo>, // Must exist as long as ximage
}

/// Pre-render wallpaper-frames for all needed resolutions, determined by
/// actual screens, options and image-data
pub fn render_wallpapers(
    xcontext: &Box<XContext>,
    xscreens: Screens,
    options: Arc<Options>,
    running: Arc<AtomicBool>,
) -> Wallpapers {
    // Decode gif-frames into raster-steps
    let path_to_gif = options.path_to_gif.as_str();

    // TODO Try using only low-level frames
    // TODO Prevent double-encoding, by re-using iterator?
    let methods = gather_disposal_methods(path_to_gif);

    // Determine image-resolution
    let first_step_result = create_decoder(path_to_gif)
        .into_steps()
        .nth(0)
        .expect("No steps decoded");

    if first_step_result.is_err() {
        eprintln!(
            "File {} is not a valid GIF: {:?}",
            path_to_gif,
            first_step_result.err().unwrap()
        );
        std::process::exit(EXIT_INVALID_FILE);
    }

    let first_step = first_step_result.unwrap();
    let raster = first_step.raster();
    let image_resolution = Resolution {
        width: raster.width(),
        height: raster.height(),
    };

    // Build wallpapers by screen
    let mut screens: Vec<WallpaperOnScreen> = Vec::new();
    let mut frames_by_resolution: HashMap<Resolution, Vec<Frame>> = HashMap::new();

    for screen in xscreens.screens {
        logln!(options, "Prepare wallpaper for {:?}", screen);

        // Gather target-resolution and image-placement for particular screen
        let screen_resolution = Resolution {
            width: screen.width,
            height: screen.height,
        };

        let target_resolution =
            image_resolution.fit_to_screen(&screen_resolution, &options.scaling);

        let wallpaper_on_screen = WallpaperOnScreen {
            placement: target_resolution.position_on_screen(&screen, Alignment::CENTER),
            resolution: target_resolution.clone(),
            _screen: screen.clone(),
        };

        // If frames were not already rendered for given resolution, do so
        if !frames_by_resolution.contains_key(&target_resolution) {
            frames_by_resolution.insert(
                target_resolution,
                render_frames(
                    xcontext,
                    &wallpaper_on_screen,
                    create_decoder(path_to_gif).into_steps().by_ref(),
                    &methods,
                    options.clone(),
                    running.clone(),
                ),
            );
        } else {
            logln!(
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
fn create_decoder(path_to_gif: &str) -> gift::Decoder<File> {
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
    wallpaper_on_screen: &WallpaperOnScreen,
    steps: &mut gift::decode::Steps<File>,
    methods: &Vec<gift::block::DisposalMethod>,
    options: Arc<Options>,
    running: Arc<AtomicBool>,
) -> Vec<Frame> {
    let mut rendered_frames: Vec<Frame> = Vec::new();
    let mut frame_index = 0;

    let xscreen = unsafe { XDefaultScreenOfDisplay(xcontext.display) };
    let xvisual = unsafe { XDefaultVisualOfScreen(xscreen) };

    // Convert rasters to frames
    for step_option in steps.by_ref() {
        if !running.load(Ordering::SeqCst) {
            break;
        }

        let step = step_option.expect("Unexpected end of steps");
        let raster = step.raster();

        let image_resolution = Resolution {
            width: raster.width(),
            height: raster.height(),
        };

        let target_resolution = wallpaper_on_screen.resolution.clone();

        logln!(
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

        let color = xcontext.background_color;
        let background_rgba = [
            (color.red / 256) as u8,
            (color.green / 256) as u8,
            (color.blue / 256) as u8,
            255 as u8,
        ];

        // Get previous frame as raster or plain color pane, if non-existing
        let prev_raster: Rc<Vec<u8>> = {
            if rendered_frames.len() > 0 {
                rendered_frames.last().unwrap().raster.clone()
            } else {
                let capacity: usize = raster.width() as usize
                    * raster.height() as usize
                    * std::mem::size_of::<Rgba8>();

                // TODO Too big, needs only the size of original frame
                let mut solid_color: Vec<u8> = Vec::with_capacity(capacity);
                let mut solid_color_index: usize = 0;

                while solid_color_index < capacity {
                    solid_color.push(background_rgba[rgba_indices[0]]);
                    solid_color.push(background_rgba[rgba_indices[1]]);
                    solid_color.push(background_rgba[rgba_indices[2]]);
                    solid_color.push(background_rgba[rgba_indices[3]]);

                    solid_color_index += 4;
                }

                Rc::new(solid_color)
            }
        };

        let u8_slice = raster.as_u8_slice();
        let mut i = 0;

        let mut data: Vec<u8> = Vec::with_capacity(image_byte_size);
        while i < u8_slice.len() {
            let alpha = u8_slice[i + 3];

            if alpha == 255 {
                data.push(u8_slice[i + rgba_indices[0]]);
                data.push(u8_slice[i + rgba_indices[1]]);
                data.push(u8_slice[i + rgba_indices[2]]);
                data.push(u8_slice[i + rgba_indices[3]]);
            } else if methods[frame_index] == gift::block::DisposalMethod::Keep {
                data.push(prev_raster[i + 0]);
                data.push(prev_raster[i + 1]);
                data.push(prev_raster[i + 2]);
                data.push(prev_raster[i + 3]);
            } else {
                data.push(background_rgba[rgba_indices[0]]);
                data.push(background_rgba[rgba_indices[1]]);
                data.push(background_rgba[rgba_indices[2]]);
                data.push(alpha);
            }

            i += 4;
        }

        let frame_ptr = Rc::new(data);
        let resized_frame = resize_raster(
            frame_ptr.clone(),
            &image_resolution,
            &target_resolution,
            options.clone(),
        );

        logln!(
            options,
            "Resized {}={}",
            resized_frame.len(),
            image_byte_size
        );

        // Copy raw data into shared memory segment of XImage
        unsafe {
            libc::memcpy(
                xshminfo.shmaddr as *mut c_void,
                resized_frame.as_ptr().clone() as *mut _,
                image_byte_size,
            );
            (*ximage).data = xshminfo.shmaddr;
            x11::xshm::XShmAttach(xcontext.display, xshminfo.as_mut() as *mut _);
        };

        rendered_frames.push(Frame {
            delay: get_step_duration(&step, options.clone()),
            raster: frame_ptr,
            ximage: unsafe { Box::new(*ximage) },
            xshminfo,
        });

        frame_index = frame_index + 1;
    }

    rendered_frames
}

/// Get delay from step. Normalize or use default if not below zere or not
/// given.
fn get_step_duration(step: &gift::Step, options: Arc<Options>) -> time::Duration {
    let mut delay = step.delay_time_cs().unwrap_or(options.default_delay);

    if delay <= 0 {
        delay = options.default_delay;
    }

    time::Duration::from_millis((delay * 10) as u64)
}

/// Resize given RGBA-raster to target-resolution.
fn resize_raster(
    raster: Rc<Vec<u8>>,
    image_resolution: &Resolution,
    target_resolution: &Resolution,
    options: Arc<Options>,
) -> Rc<Vec<u8>> {
    let src_w = image_resolution.width as usize;
    let src_h = image_resolution.height as usize;
    let dst_w = target_resolution.width as usize;
    let dst_h = target_resolution.height as usize;

    let must_resize = src_w != dst_w || src_h != dst_h;

    if !must_resize {
        return raster.clone();
    }

    let (resize_type, type_name) = match options.scaling_filter {
        ScalingFilter::PIXEL => (resize::Type::Point, "Point"),
        ScalingFilter::AUTO => {
            if (src_w * src_h) > (dst_w * dst_h) {
                (resize::Type::Lanczos3, "Lanczos3")
            } else {
                (resize::Type::Mitchell, "Mitchell")
            }
        }
    };

    logln!(
        options,
        "Resize raster from {}x{} to {}x{}, using filter {}",
        src_w,
        src_h,
        dst_w,
        dst_h,
        type_name
    );

    let sample_size = dst_w * dst_h * 4;

    let mut dst: Vec<u8> = Vec::with_capacity(sample_size);
    dst.resize(sample_size, 0);

    let mut resizer = resize::new(src_w, src_h, dst_w, dst_h, resize::Pixel::RGBA, resize_type);

    resizer.resize(&raster, &mut dst);

    Rc::new(dst)
}

/// Clear previous backgrounds on root.
pub fn clear_background(xcontext: &Box<XContext>, options: Arc<Options>) {
    remove_root_pixmap_atoms(&xcontext, options.clone());

    unsafe {
        XClearWindow(xcontext.display, xcontext.root);
        XSync(xcontext.display, False);
    }
}

/// Loops the pre-renders wallpapers on each screen. Will only stop on
/// interrupt-signal.
pub fn do_animation(
    xcontext: &Box<XContext>,
    wallpapers: &mut Wallpapers,
    options: Arc<Options>,
    running: Arc<AtomicBool>,
) {
    logln!(options, "Loop animation...");

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

            //logln!(options, "Put frame {} on screen {:?}", i, screen.placement);

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

    logln!(options, "Stop animation-loop");

    delete_atom(&xcontext, atom_root);
    delete_atom(&xcontext, atom_eroot);
}

/// Clears reference and (shared-)-memory.
pub fn clean_up(xcontext: Box<XContext>, mut wallpapers: Wallpapers, options: Arc<Options>) {
    logln!(options, "Free images in shared memory");

    for frames in wallpapers.frames_by_resolution.values_mut() {
        for i in 0..(frames.len()) {
            // Don't need to call XDestroy image - heap is freed by rust-guarantees. :)
            unsafe {
                x11::xshm::XShmDetach(xcontext.display, frames[i].xshminfo.as_mut() as *mut _)
            };
            destroy_xshm_sgmnt_inf(&mut frames[i].xshminfo);
        }
    }
}

#[cfg(all(test, feature = "x11-integration-tests"))]
mod test {
    /// Test for github-issue #3:
    /// [https://github.com/calculon102/xgifwallpaper/issues/3]
    #[test]
    fn when_render_for_multiple_resolutions_then_dont_panic() {
        // Prepare
        use crate::options::Options;
        use crate::render_wallpapers;
        use crate::screens::*;
        use crate::xcontext::XContext;
        use std::sync::atomic::AtomicBool;
        use std::sync::Arc;

        let options = Arc::new(Options::_from_params(vec![
            "xgifwallpaper",
            "--scale",
            "FILL",
            "tests/samples/sample-1x1.gif",
        ]));

        let xcontext = Box::new(XContext::new(options.clone()).unwrap());
        let running = Arc::new(AtomicBool::new(true));

        let screens = Screens {
            root_per_screen: false,
            screens: vec![
                Screen {
                    screen_number: 0,
                    x_org: 0,
                    y_org: 0,
                    width: 800,
                    height: 600,
                },
                Screen {
                    screen_number: 1,
                    x_org: 400,
                    y_org: 300,
                    width: 1920,
                    height: 1080,
                },
            ],
        };

        // Act
        let wallpapers = render_wallpapers(&xcontext, screens, options.clone(), running.clone());

        assert_eq!(wallpapers.screens.len(), 2);
        assert_eq!(wallpapers.frames_by_resolution.len(), 2);
    }
}
