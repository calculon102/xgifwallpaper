mod placement;
mod screen_info;

use clap::{App, Arg, ArgMatches};

use gift::decode::Steps;
use gift::Decoder;

use pix::rgb::Rgba8;

use std::ffi::{c_void, CString};
use std::fs::File;
use std::io::BufReader;
use std::mem;
use std::os::raw::{c_char, c_int, c_uint};
use std::ptr::{null, null_mut};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::{thread, time};

use x11::xlib::*;
use x11::xshm;

use placement::*;
use screen_info::*;

// TODO Refactor set_root_atoms as two functions
// TODO default-delay as argument
// TODO background-color as argument
// TODO placement as argument

struct Frame {
    delay: time::Duration,
    placements: Vec<ImagePlacement>,
    raster: Rc<Vec<c_char>>,
    ximage: Box<XImage>,
    _xshminfo: Box<xshm::XShmSegmentInfo>, // Must exist as long ximage is used
}

struct Options<'a> {
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
        let display = XOpenDisplay(null());

        let r = running.clone();
        let mut frames = prepare_frames(options.clone(), r, display, steps);

        // Single root-loop
        // TODO multi-root implementation
        let screen = XDefaultScreen(display);
        let gc = XDefaultGC(display, screen);
        let display_width = XDisplayWidth(display, screen);
        let display_height = XDisplayHeight(display, screen);
        let root = XRootWindow(display, screen);
        let depth = XDefaultDepth(display, screen) as u32;

        let pixmap = XCreatePixmap(
            display,
            root,
            display_width as u32,
            display_height as u32,
            depth,
        );

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

        while running.load(Ordering::SeqCst) {
            for i in 0..(frames.len()) {
                if !running.load(Ordering::SeqCst) {
                    break;
                }

                for j in 0..(frames[i].placements.len()) {
                    xshm::XShmPutImage(
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

                if !set_root_atoms(display, root, pixmap) {
                    println!("set_root_atoms failed!");
                }

                XSetWindowBackgroundPixmap(display, root, pixmap);
                XSync(display, False);

                thread::sleep(frames[i].delay);
            }
        }

        // Clean up
        for i in 0..(frames.len()) {
            // Don't need to call XDestroy image - heap is freed by rust-guarantees. :)
            xshm::XShmDetach(display, frames[i]._xshminfo.as_mut() as *mut _);
            destroy_xshm_sgmnt_inf(&mut frames[i]._xshminfo);
        }

        XFreePixmap(display, pixmap);
        XCloseDisplay(display);
    }
}

fn prepare_frames(
    options: Arc<Options>,
    running: Arc<AtomicBool>,
    xdisplay: *mut Display,
    frames: Steps<BufReader<File>>,
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

        let mut i = 0;
        let s = 4;

        let prev_raster: Rc<Vec<i8>> = {
            if out.len() > 0 {
                out.last().unwrap().raster.clone()
            } else {
                // TODO User-defined background-color
                Rc::new(vec![
                    0;
                    raster.width() as usize
                        * raster.height() as usize
                        * std::mem::size_of::<Rgba8>()
                ])
            }
        };

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
            xshm::XShmAttach(xdisplay, xshminfo.as_mut() as *mut _);
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
            _xshminfo: xshminfo,
        });
    }

    return out;
}

fn create_xshm_sgmnt_inf(data: Rc<Vec<i8>>, size: usize) -> Result<Box<xshm::XShmSegmentInfo>, u8> {
    use libc::size_t;
    let shmid: c_int =
        unsafe { libc::shmget(libc::IPC_PRIVATE, size as size_t, libc::IPC_CREAT | 0o777) };
    if shmid < 0 {
        return Err(1);
    }
    let shmaddr: *mut libc::c_void = unsafe { libc::shmat(shmid, null(), 0) };
    if shmaddr == ((usize::max_value()) as *mut libc::c_void) {
        return Err(2);
    }
    let mut shmidds: libc::shmid_ds = unsafe { mem::zeroed() };
    unsafe { libc::shmctl(shmid, libc::IPC_RMID, &mut shmidds) };

    unsafe { libc::memcpy(shmaddr as *mut c_void, data.as_ptr() as *mut _, size) };

    Ok(Box::new(xshm::XShmSegmentInfo {
        shmseg: 0,
        shmid,
        shmaddr: (shmaddr as *mut c_char),
        readOnly: 0,
    }))
}

fn destroy_xshm_sgmnt_inf(seginf: &mut Box<xshm::XShmSegmentInfo>) {
    unsafe { libc::shmdt(seginf.shmaddr as *mut libc::c_void) };
}

fn create_xshm_image(
    dspl: *mut Display,
    vsl: *mut Visual,
    xshminfo: &mut Box<xshm::XShmSegmentInfo>,
    width: u32,
    height: u32,
    depth: u32,
) -> Result<*mut XImage, u8> {
    unsafe {
        let ximg = xshm::XShmCreateImage(
            dspl,
            vsl,
            depth,
            ZPixmap,
            null_mut(),
            xshminfo.as_mut() as *mut _,
            width,
            height,
        );
        if ximg == null_mut() {
            return Err(1);
        }
        (*ximg).data = xshminfo.shmaddr;
        Ok(ximg)
    }
}

// TODO Split into update atoms and remove old atoms
unsafe fn set_root_atoms(display: *mut Display, root: u64, pixmap: Pixmap) -> bool {
    let xrootmap_id = CString::new("_XROOTPMAP_ID").expect("Failed!");
    let esetroot_pmap_id = CString::new("ESETROOT_PMAP_ID").expect("Failed!");

    let mut atom_root = XInternAtom(display, xrootmap_id.as_ptr(), True);
    let mut atom_eroot = XInternAtom(display, esetroot_pmap_id.as_ptr(), True);

    // Doing this to clean up after old background.
    //
    // XInternAtom may return "None", but nowhere defined in bindigs? So I
    // use 0 as direct, known value of None. See X.h.
    if atom_root != 0 && atom_eroot != 0 {
        // TODO Better way to have an initialized, non-null pointer?
        let data_root = CString::new("").expect("Failed!");
        let mut data_root_ptr: *mut u8 = data_root.as_ptr() as *mut u8;

        let data_eroot = CString::new("").expect("Failed!");
        let mut data_eroot_ptr: *mut u8 = data_eroot.as_ptr() as *mut u8;

        let mut ptype = 0 as u64;
        let mut format = 0 as i32;
        let mut length = 0 as u64;
        let mut after = 0 as u64;

        let result = XGetWindowProperty(
            display,
            root,
            atom_root,
            0,
            1,
            False,
            AnyPropertyType as u64,
            &mut ptype,
            &mut format,
            &mut length,
            &mut after,
            &mut data_root_ptr,
        );

        if result == Success as i32 && ptype == XA_PIXMAP {
            XGetWindowProperty(
                display,
                root,
                atom_eroot,
                0,
                1,
                0,
                AnyPropertyType as u64,
                &mut ptype,
                &mut format,
                &mut length,
                &mut after,
                &mut data_eroot_ptr,
            );

            let root_pixmap_id = *(data_root_ptr as *const Pixmap);
            let eroot_pixmap_id = *(data_eroot_ptr as *const Pixmap);

            if ptype == XA_PIXMAP && root_pixmap_id == eroot_pixmap_id && pixmap != root_pixmap_id {
                // Don't kill myself

                println!(
                    "Kill client responsible for _XROOTPMAP_ID {}",
                    root_pixmap_id
                );

                //                XKillClient(display, root_pixmap_id);
                XFree(data_eroot_ptr as *mut c_void);
            }

            XFree(data_root_ptr as *mut c_void);
        }
    }

    atom_root = XInternAtom(display, xrootmap_id.as_ptr(), 0);
    atom_eroot = XInternAtom(display, esetroot_pmap_id.as_ptr(), 0);

    if atom_root == 0 || atom_eroot == 0 {
        return false;
    }

    // setting new background atoms
    let pixmap_ptr: *const Pixmap = &pixmap;

    XChangeProperty(
        display,
        root,
        atom_root,
        XA_PIXMAP,
        32,
        PropModeReplace,
        pixmap_ptr as *const u8,
        1,
    );
    XChangeProperty(
        display,
        root,
        atom_eroot,
        XA_PIXMAP,
        32,
        PropModeReplace,
        pixmap_ptr as *const u8,
        1,
    );

    return true;
}

fn main() {
    let args = init_args();

    let options = Arc::new(Options {
        path_to_gif: args.value_of("PATH_TO_GIF").unwrap(),
        verbose: args.is_present("VERBOSE"),
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
        .about("Animates GIF as background in your X-session")
        .arg(Arg::with_name("VERBOSE").short("v").help("Verbose mode"))
        .arg(
            Arg::with_name("PATH_TO_GIF")
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
