extern crate x11;

use std::ffi::CString;
use std::ptr;
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

fn main () {
    unsafe {
        // Open display connection.
        let mut display = xlib::XOpenDisplay(ptr::null());

        if display.is_null() {
            panic!("XOpenDisplay failed");
        }

        // Create window.
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
            plane_mask: 0,
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
            subwindow_mode: 0,
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
        
        println!("display: {:?}", display);
        println!("screen: {}", screen);
        println!("depth: {}", depth);
        println!("width: {}", width);
        println!("height: {}", height);
        println!("color.pixel: {}", color.pixel);

        if !set_root_atoms(display, root, pixmap) {
            println!("set_root_atoms failed!");
        }

        display = xlib::XOpenDisplay(ptr::null());

        xlib::XKillClient(display, xlib::AllTemporary as u64);

        xlib::XSetWindowBackgroundPixmap(display, root, pixmap);
        xlib::XClearWindow(display, root);
        // TODO Overdraws every other window
        // TODO No effect on composited status bar?
        xlib::XCopyArea(display, pixmap, root, gc, 0, 0, width, height, 0, 0);

        xlib::XFlush(display);
        xlib::XSync(display, 0);


        xlib::XFreePixmap(display, pixmap);
        xlib::XFreeGC(display, gc);
        xlib::XCloseDisplay(display);
    }
}

// Adapted from hsetroot
unsafe fn set_root_atoms(display: *mut xlib::Display, root: u64, pixmap: xlib::Pixmap) -> bool {
    println!("set_root_atoms: display: {:?}", display);
    
    let xrootmap_id = CString::new("_XROOTMAP_ID").expect("Failed!"); 
    let esetroot_pmap_id = CString::new("_ESETROOT_PMAP_ID").expect("Failed!"); 

    let mut atom_root = xlib::XInternAtom(display, xrootmap_id.as_ptr(), 1);
    let mut atom_eroot = xlib::XInternAtom(display, esetroot_pmap_id.as_ptr(), 1);

    println!("Atoms: {} {}", atom_root, atom_eroot);

    // Doing this to clean up after old background.
    //
    // XInternAtom may return "None", but nowhere defined in bindigs? So I
    // use 0 as direct, known value of None. See X.h.
    if atom_root != 0 && atom_eroot != 0 {
        // TODO Better way to have an initialized, non-null pointer?
        let data_root = CString::new("00000000").expect("Failed!"); 
        let mut data_root_ptr : *mut u8 = data_root.as_ptr() as *mut u8;

        let data_eroot = CString::new("00000000").expect("Failed!");
        let mut data_eroot_ptr : *mut u8 = data_eroot.as_ptr() as *mut u8;

        let mut ptype = 0 as u64;
        let mut format = 0 as i32;
        let mut length = 0 as u64;
        let mut after = 0 as u64;

        println!("data_root: {}, data_eroot: {}, ptype: {}, format: {}, length: {}, after: {}", *data_root_ptr, *data_eroot_ptr, ptype, format, length, after);
        
        let result = xlib::XGetWindowProperty(display, root, atom_root, 0, 1, 0, xlib::AnyPropertyType as u64, &mut ptype, &mut format, &mut length, &mut after, &mut data_root_ptr);

        if result == xlib::True && ptype == xlib::XA_PIXMAP {
            xlib::XGetWindowProperty(display, root, atom_eroot, 0, 1, 0, xlib::AnyPropertyType as u64, &mut ptype, &mut format, &mut length, &mut after, &mut data_eroot_ptr);

            println!("data_root: {}, data_eroot: {}, ptype: {}, format: {}, length: {}, after: {}", *data_root_ptr, *data_eroot_ptr, ptype, format, length, after);

            // Why the data_root-conversion to pixmap for equality-check???
            if // *data_root > 0 
               //  && *data_eroot > 0 
                 ptype == xlib::XA_PIXMAP 
                && *data_root == *data_eroot {
                
                let old_pixmap_ptr = data_root_ptr as *const xlib::Pixmap;
                println!("old_pixmap_ptr: {}", *old_pixmap_ptr);

                let kill_result = xlib::XKillClient(display, *old_pixmap_ptr);

                println!("After XKillClient: {}", kill_result);
            }
        }
    }

    atom_root = xlib::XInternAtom(display, xrootmap_id.as_ptr(), 0);
    atom_eroot = xlib::XInternAtom(display, esetroot_pmap_id.as_ptr(), 0);

    println!("Atoms: {} {}", atom_root, atom_eroot);
    
    if atom_root == 0 || atom_eroot == 0 {
        return false;
    }

    // setting new background atoms
    let pixmap_ptr: *const xlib::Pixmap = &pixmap;
    println!("pixmap_ptr: {}", *pixmap_ptr);
    
    let change_result1 = xlib::XChangeProperty(display, root, atom_root, xlib::XA_PIXMAP, 32, xlib::PropModeReplace, pixmap_ptr as *const u8, 1);
    let change_result2 = xlib::XChangeProperty(display, root, atom_eroot, xlib::XA_PIXMAP, 32, xlib::PropModeReplace, pixmap_ptr as *const u8, 1);

    xlib::XFlush(display);

    println!("Result: {}, {}", change_result1, change_result2);

    return true;
}
