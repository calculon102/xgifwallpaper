//! X11-specific control-data, references and connection-handling.

use std::ffi::CString;
use std::os::raw::{c_int, c_uint, c_ulong};
use std::ptr;
use std::sync::Arc;
use x11::xlib::{
    Display, FillSolid, Pixmap, XAllocColor, XCloseDisplay, XColor, XConnectionNumber,
    XCreatePixmap, XDefaultColormap, XDefaultDepth, XDefaultGC, XDefaultScreen, XDisplayHeight,
    XDisplayWidth, XDrawRectangle, XFillRectangle, XOpenDisplay, XParseColor, XRootWindow,
    XSetBackground, XSetFillStyle, XSetForeground, GC,
};

use crate::options::Options;
use crate::shm::is_xshm_available;
use crate::EXIT_NO_XDISPLAY;
use crate::EXIT_UNKOWN_COLOR;
use crate::EXIT_XSHM_UNSUPPORTED;

/// X11-specific control-data and references.
#[derive(Debug)]
pub struct XContext {
    pub background_color: XColor,
    pub display: *mut Display,
    pub gc: GC,
    pub pixmap: Pixmap,
    pub root: c_ulong,
    pub screen: c_int,
}

impl XContext {
    /// Start the X11-lifecycle:
    ///
    /// * Creates a connection to the default display of X
    /// * Checks if XSHM is available, exits the process otherweise
    /// * Queries defaults for screen, gc and root window
    /// * Parses given color in option as X11-color
    /// * Prepares the pixmap for frame-drawing
    /// * Parses the option for alternate window-id, than root
    pub fn new(opts: Arc<Options>) -> XContext {
        let display = unsafe { XOpenDisplay(ptr::null()) };

        log!(opts, "Open X-display: ");

        if display.is_null() {
            eprintln!("Failed to open display. Is X running in your session?");
            std::process::exit(EXIT_NO_XDISPLAY);
        }

        if !is_xshm_available(display) {
            eprintln!("The X server in use does not support the shared memory extension (xshm).");
            std::process::exit(EXIT_XSHM_UNSUPPORTED);
        }

        logln!(opts, "connection-number={:?}", unsafe {
            XConnectionNumber(display)
        });

        log!(opts, "Query context from X server: ");

        let screen = unsafe { XDefaultScreen(display) };
        let gc = unsafe { XDefaultGC(display, screen) };
        let root = if opts.window_id > 0 {
            opts.window_id
        } else {
            unsafe { XRootWindow(display, screen) }
        };

        logln!(
            opts,
            "DefaultScreen={:?}, DefaultGC={:?}, RootWindow={:?}",
            screen,
            gc,
            root
        );

        let background_color = parse_color(display, screen, opts.clone());
        let pixmap = prepare_pixmap(display, screen, gc, root, &background_color);

        XContext {
            background_color,
            display,
            gc,
            pixmap,
            root,
            screen,
        }
    }
}

/// Parse string as X11-color.
fn parse_color(display: *mut Display, screen: c_int, opts: Arc<Options>) -> XColor {
    let mut xcolor: XColor = XColor {
        pixel: 0,
        red: 0,
        green: 0,
        blue: 0,
        flags: 0,
        pad: 0,
    };

    let xcolor_ptr: *mut XColor = &mut xcolor;
    let color_str = opts.background_color.as_str();
    let cmap = unsafe { XDefaultColormap(display, screen) };

    log!(opts, "Parse \"{}\" as X11-color: ", color_str);

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

    logln!(opts, "{:?}", xcolor);

    xcolor
}

/// Create and prepare the pixmap, where the wallpaper is drawn onto.
fn prepare_pixmap(
    dsp: *mut Display,
    scr: c_int,
    gc: GC,
    root: c_ulong,
    background_color: &XColor,
) -> Pixmap {
    unsafe {
        let dsp_width = XDisplayWidth(dsp, scr) as c_uint;
        let dsp_height = XDisplayHeight(dsp, scr) as c_uint;
        let depth = XDefaultDepth(dsp, scr) as c_uint;

        let pixmap = XCreatePixmap(dsp, root, dsp_width, dsp_height, depth);

        XSetForeground(dsp, gc, background_color.pixel);
        XSetBackground(dsp, gc, background_color.pixel);
        XSetFillStyle(dsp, gc, FillSolid);

        XDrawRectangle(dsp, pixmap, gc, 0, 0, dsp_width, dsp_height);
        XFillRectangle(dsp, pixmap, gc, 0, 0, dsp_width, dsp_height);

        pixmap
    }
}
