extern crate x11;

use image::gif::{GifDecoder};
use image::{AnimationDecoder, Frame};

use std::ptr;


use std::ffi:: {
    CString,
    c_void
};
use std::fs::File;
use x11::xlib;

// TODO Show XBM as background
// TODO Load GIF
// TODO Convert GIF-Frames to XBM
// TODO Set XBM as Background in tempo of GIF
// TODO Center
// TODO Scale
// TODO Fill?
// TODO On all Screens
// TODO Multiple images on different screens
// TODO Minimize unsafe
// TODO cmd-opts
// TODO Verbose mode

fn load_gif(filename: String) -> Vec<Frame>
{
    let file_in = File::open(filename)
        .expect("Could not load gif");

    let decoder = GifDecoder::new(file_in)
        .expect("Error initializing gif-decoder");

    let frames = decoder.into_frames();

    return frames.collect_frames()
        .expect("error decoding gif");
}

fn main() {
    // TODO Read Args
    let gif_filename = String::from("/home/frank/Pictures/Wallpapers/2020-gifs/pixels4.gif");
    
    // TODO Analyze screen-count and resolutions
   
    // TODO Load GIF
    let frames = load_gif(gif_filename);

    let mut count = 0;

    for frame in frames.iter() {
        count = count + 1;
        println!("Frame {}", count);
        println!("delay: {:?}", frame.delay());
        println!("dimensions: {:?}", frame.buffer().dimensions());
        println!("left: {:?}", frame.left());
        println!("top: {:?}", frame.top());
    }

    // TODO Scale GIF-Frames accordingly to params (Center, Scale, Fill)
    
    // TODO Render frames as background
    
    // TODO Animation-Loop

    unsafe {
//        demo();
    }
}

unsafe fn demo() 
{
    // Open display connection.
    let display = xlib::XOpenDisplay(ptr::null());

    if display.is_null() {
        panic!("XOpenDisplay failed");
    }

    println!("ScreenCount: {}", xlib::XScreenCount(display));

    let screen = xlib::XDefaultScreen(display);
    let width = xlib::XDisplayWidth(display, screen) as u32;
    let height = xlib::XDisplayHeight(display, screen) as u32;
    let root = xlib::XRootWindow(display, screen);
    let cmap = xlib::XDefaultColormap(display, screen);
    let depth = xlib::XDefaultDepth(display, screen) as u32;

    let mut color = xlib::XColor {
        pixel: 0,
        red: 32000,
        green: 64000,
        blue: 32000,
        flags: xlib::DoRed | xlib::DoGreen | xlib::DoBlue,
        pad: 0
    };

    let color_ptr: *mut xlib::XColor = &mut color;

    println!("display: {:?}", display);

    xlib::XAllocColor(display, cmap, color_ptr);

    println!("display: {:?}", display);
    
    let pixmap = xlib::XCreatePixmap(display, root, width, height, depth);

    println!("display: {:?}", display);

    let mut gcvalues = xlib::XGCValues {
        function: xlib::GXcopy,
        plane_mask: xlib::XAllPlanes(),
        foreground: color.pixel,
        background: color.pixel,
        line_width: 0,
        line_style: 0,
        cap_style: 0,
        join_style: 0,
        fill_style: 0,
        fill_rule: 0,
        arc_mode: 0,
        tile: 0,
        stipple: 0,
        ts_x_origin: 0,
        ts_y_origin: 0,
        font: 0,
        subwindow_mode: xlib::ClipByChildren,
        graphics_exposures: xlib::True,
        clip_x_origin: 0,
        clip_y_origin: 0,
        clip_mask: 0,
        dash_offset: 0,
        dashes: 0,
    };
    
    let gc_ptr: *mut xlib::XGCValues = &mut gcvalues;
    let gc_flags = (xlib::GCForeground | xlib::GCBackground) as u64;
    let gc = xlib::XCreateGC(display, root, gc_flags, gc_ptr);

    xlib::XFillRectangle(display, pixmap, gc, 0, 0, width, height);
    xlib::XFreeGC(display, gc);
    
    println!("display: {:?}", display);
    println!("screen: {}", screen);
    println!("depth: {}", depth);
    println!("width: {}", width);
    println!("height: {}", height);
    println!("color.pixel: {}", color.pixel);

    if !set_root_atoms(display, root, pixmap) {
        println!("set_root_atoms failed!");
    }

    xlib::XSetWindowBackgroundPixmap(display, root, pixmap);
    xlib::XClearWindow(display, root);
    xlib::XFlush(display);
    xlib::XSetCloseDownMode(display, xlib::RetainPermanent);
    xlib::XCloseDisplay(display);
}

unsafe fn set_root_atoms(display: *mut xlib::Display, root: u64, pixmap: xlib::Pixmap) -> bool {
    let xrootmap_id = CString::new("_XROOTPMAP_ID").expect("Failed!"); 
    let esetroot_pmap_id = CString::new("ESETROOT_PMAP_ID").expect("Failed!"); 

    let mut atom_root = xlib::XInternAtom(display, xrootmap_id.as_ptr(), xlib::True);
    let mut atom_eroot = xlib::XInternAtom(display, esetroot_pmap_id.as_ptr(), xlib::True);

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

        let result = xlib::XGetWindowProperty(display, root, atom_root, 0, 1, xlib::False, xlib::AnyPropertyType as u64, &mut ptype, &mut format, &mut length, &mut after, &mut data_root_ptr);

        if result == xlib::Success as i32 && ptype == xlib::XA_PIXMAP {
            xlib::XGetWindowProperty(display, root, atom_eroot, 0, 1, 0, xlib::AnyPropertyType as u64, &mut ptype, &mut format, &mut length, &mut after, &mut data_eroot_ptr);

            let root_pixmap_id = *(data_root_ptr as *const xlib::Pixmap);
            let eroot_pixmap_id = *(data_eroot_ptr as *const xlib::Pixmap);

            // Why the data_root-conversion to pixmap for equality-check???
            if // *data_root > 0 
               //  && *data_eroot > 0 
                 ptype == xlib::XA_PIXMAP 
                && root_pixmap_id == eroot_pixmap_id {

                xlib::XKillClient(display, root_pixmap_id);
                xlib::XFree(data_eroot_ptr as *mut c_void);
            }

            xlib::XFree(data_root_ptr as *mut c_void);
        }
    }

    atom_root = xlib::XInternAtom(display, xrootmap_id.as_ptr(), 0);
    atom_eroot = xlib::XInternAtom(display, esetroot_pmap_id.as_ptr(), 0);

    if atom_root == 0 || atom_eroot == 0 {
        return false;
    }

    // setting new background atoms
    let pixmap_ptr: *const xlib::Pixmap = &pixmap;
    println!("pixmap_ptr: {}", *pixmap_ptr);
    
    xlib::XChangeProperty(display, root, atom_root, xlib::XA_PIXMAP, 32, xlib::PropModeReplace, pixmap_ptr as *const u8, 1);
    xlib::XChangeProperty(display, root, atom_eroot, xlib::XA_PIXMAP, 32, xlib::PropModeReplace, pixmap_ptr as *const u8, 1);

    return true;
}
