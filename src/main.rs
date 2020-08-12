mod placement;
mod screen_info;

use gift::Decoder;
use gift::decode::Steps;

use std::ptr;
use std::ffi:: { CString, c_void };
use std::fs::File;
use std::io::BufReader;
use std::os::raw:: { c_char, c_uint };
use std::{ thread, time };

use x11::xlib::*;

use screen_info::*;
use placement::*;

// TODO Include Signal handler and loop indefinetly
// TODO Introduce cmd-opts, reading file to render from arg
// TODO Find lib to Fill and Scale-Feature images
// TODO Introduce libxshm for better performance, see
//  https://stackoverflow.com/questions/23873175/how-to-efficiently-draw-framebuffer-content
//  https://www.x.org/releases/current/doc/xextproto/shm.html
// TODO Refactor set_root_atoms as two functions
// TODO Verbose mode

struct Frame {
    image: XImage,
    _raster: Vec<c_char>,
    delay: time::Duration,
    placements: Vec<ImagePlacement>
}


fn load_gif(filename: String) -> Steps<BufReader<File>>
{
    let file_in = File::open(filename)
        .expect("Could not load gif");

    let decoder = Decoder::new(file_in);

    return decoder.into_steps();
}

fn loop_animation(steps: Steps<BufReader<File>>)
{
    unsafe {
        let display = XOpenDisplay(ptr::null());
        
        let screen_info = get_screen_info();

        let mut frames = prepare_frames(display, steps);
        
        // Single root-loop
        // TODO multi-root implementation
        let screen = XDefaultScreen(display);
        let gc = XDefaultGC(display, screen);
        let display_width = XDisplayWidth(display, screen);
        let display_height = XDisplayHeight(display, screen);
        let root = XRootWindow(display, screen);
        let depth = XDefaultDepth(display, screen) as u32;

        let pixmap = XCreatePixmap(display, root, display_width as u32, display_height as u32, depth);

        XClearWindow(display, root);
        XSync(display, False);
               
        for i in 0..(frames.len()) {
            let image_width = frames[i].image.width;
            let image_height = frames[i].image.height;

            for screen in &screen_info.screens {
                frames[i].placements.push(
                    get_image_placement(
                        image_width,
                        image_height,
                        screen.clone(),
                        ImagePlacementStrategy::CENTER,
                    )
                );
            }
        }

        for _x in 0..10 {
//        loop {
            for frame in &frames {
                let mut image = frame.image;

                for placement in &frame.placements {
                    XPutImage(
                        display,
                        pixmap,
                        gc,
                        &mut image,
                        placement.src_x,
                        placement.src_y,
                        placement.dest_x,
                        placement.dest_y,
                        placement.width as c_uint, 
                        placement.height as c_uint
                    );
                }

                if !set_root_atoms(display, root, pixmap) {
                    println!("set_root_atoms failed!");
                }

                XSetWindowBackgroundPixmap(display, root, pixmap);
      
                XSync(display, False);

                thread::sleep(frame.delay);
            }
        }

        // TODO React on signal in loop
        XFreePixmap(display, pixmap);
        XCloseDisplay(display);
    }
}

fn prepare_frames(xdisplay: *mut Display, frames: Steps<BufReader<File>>) -> Vec<Frame>
{
    let mut out: Vec<Frame> = Vec::new();

    let mut frame_count = 0;
    
    for step_option in frames {
        let step = step_option.expect("Empty step in animation");
        let raster = step.raster();

        frame_count = frame_count + 1;
        println!("Step: {}", frame_count);
        println!("Delay: {:?}", step.delay_time_cs());
        println!("Width: {}", raster.width());
        println!("Height: {}", raster.height());

        unsafe {
            let xscreen = XDefaultScreenOfDisplay(xdisplay);
            let xvisual = XDefaultVisualOfScreen(xscreen);

            let ximage = XCreateImage(
                xdisplay,
                xvisual,
                24,
                ZPixmap,
                0,
                ptr::null_mut(), 
                raster.width(),
                raster.height(),
                32,
                0 
            );

            let data_size = ((*ximage).bytes_per_line * (*ximage).height) as usize;

            // Have to copy slice to make available to xlib-struct.
            // Would be better of, making a pointer to slice-data, still.
            let i8_slice = &*(raster.as_u8_slice() as *const [u8] as *const [i8]);
            let mut data = i8_slice.to_vec();

            assert_eq!(data.len(), data_size, 
                "data-vector must be same length (is {}) as its anticipated capacity and size (is {})", 
                data.len(), data_size);

            let data_ptr = data.as_mut_ptr();
            (*ximage).data = data_ptr;       
   
            let mut delay = step.delay_time_cs().unwrap_or(10);
            if delay <= 0  {
                delay = 10;
            }

            out.push(Frame {
                image: *ximage,
                _raster: data,
                delay: time::Duration::from_millis((delay * 10) as u64), 
                placements: Vec::new(),
            });
        }
    }

    return out;
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
        let mut data_root_ptr : *mut u8 = data_root.as_ptr() as *mut u8;

        let data_eroot = CString::new("").expect("Failed!");
        let mut data_eroot_ptr : *mut u8 = data_eroot.as_ptr() as *mut u8;

        let mut ptype = 0 as u64;
        let mut format = 0 as i32;
        let mut length = 0 as u64;
        let mut after = 0 as u64;

        let result = XGetWindowProperty(display, root, atom_root, 0, 1, False, AnyPropertyType as u64, &mut ptype, &mut format, &mut length, &mut after, &mut data_root_ptr);

        if result == Success as i32 && ptype == XA_PIXMAP {
            XGetWindowProperty(display, root, atom_eroot, 0, 1, 0, AnyPropertyType as u64, &mut ptype, &mut format, &mut length, &mut after, &mut data_eroot_ptr);

            let root_pixmap_id = *(data_root_ptr as *const Pixmap);
            let eroot_pixmap_id = *(data_eroot_ptr as *const Pixmap);

            if ptype == XA_PIXMAP 
                && root_pixmap_id == eroot_pixmap_id 
                && pixmap != root_pixmap_id { // Don't kill myself

                println!("Kill client responsible for _XROOTPMAP_ID {}", root_pixmap_id);

                XKillClient(display, root_pixmap_id);
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
    
    XChangeProperty(display, root, atom_root, XA_PIXMAP, 32, PropModeReplace, pixmap_ptr as *const u8, 1);
    XChangeProperty(display, root, atom_eroot, XA_PIXMAP, 32, PropModeReplace, pixmap_ptr as *const u8, 1);

    return true;
}


fn main() {
    // TODO Read Args
    //let gif_filename = String::from("/home/frank/Pictures/sample.gif");
    let gif_filename = String::from("/home/frank/Pictures/Wallpapers/2020-gifs/pixels1.gif");
    
    // Load GIF
    let steps = load_gif(gif_filename);

    // TODO Scale GIF-Frames accordingly to params (Center, Scale, Fill)
    loop_animation(steps);
}
