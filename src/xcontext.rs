//! X11-specific control-data, references and connection-handling.

use std::error::Error;
use std::ffi::CString;
use std::os::raw::{c_int, c_uint, c_ulong};
use std::ptr;
use std::result::*;
use std::sync::Arc;

use x11::xlib::{
    Display, FillSolid, Pixmap, XAllocColor, XCloseDisplay, XColor, XConnectionNumber, XClearWindow,
    XCreatePixmap, XDefaultColormap, XDefaultDepth, XDefaultGC, XDefaultScreen, XDisplayHeight,
    XDisplayWidth, XDrawRectangle, XFillRectangle, XFreePixmap, XOpenDisplay, XParseColor, XRootWindow,
    XSetBackground, XSetWindowBackground, XSetFillStyle, XSetForeground, GC,
};

use crate::options::Options;
use crate::shm::is_xshm_available;
use crate::xatoms::{get_atom, query_window_propery_as_window_id};

const EXIT_NO_XDISPLAY: i32 = 100;
const EXIT_XSHM_UNSUPPORTED: i32 = 101;
const EXIT_UNKOWN_COLOR: i32 = 102;
const EXIT_INVALID_WINDOW_ID: i32 = 104;

/// X11-specific control-data and references.
#[derive(Debug)]
pub struct XContext {
    pub background_color: XColor,
    pub display: *mut Display,
    pub gc: GC,
    pub pixmap: Pixmap,
    pub root: c_ulong,
    pub screen: c_int,
    options: Arc<Options>,
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
    pub fn new(opts: Arc<Options>) -> Result<XContext, XContextError> {
        let display = unsafe { XOpenDisplay(ptr::null()) };

        log!(opts, "Open X-display: ");

        if display.is_null() {
            return Err(XContextError::with(
                EXIT_NO_XDISPLAY,
                "Failed to open display. Is X running in your session?".to_string(),
            ));
        }

        if !is_xshm_available(display) {
            return Err(XContextError::with(
                EXIT_XSHM_UNSUPPORTED,
                "The X server in use does not support the shared memory extension (xshm)."
                    .to_string(),
            ));
        }

        logln!(opts, "connection-number={:?}", unsafe {
            XConnectionNumber(display)
        });

        log!(opts, "Query context from X server: ");

        let screen = unsafe { XDefaultScreen(display) };
        let gc = unsafe { XDefaultGC(display, screen) };
        let root = unsafe { XRootWindow(display, screen) };

        let window = if opts.window_id.is_empty() {
            root
        } else {
            parse_window_id(display, root, &opts.window_id)?
        };

        logln!(
            opts,
            "DefaultScreen={:?}, DefaultGC={:?}, RootWindow={:?}, WindowToUse={:?}",
            screen,
            gc,
            root,
            window
        );

        let background_color = parse_color(display, screen, opts.clone())?;
        let pixmap = prepare_pixmap(display, screen, gc, window, &background_color);

        Ok(XContext {
            background_color,
            display,
            gc,
            pixmap,
            root: window,
            screen,
            options: opts.clone(),
        })
    }
}

impl Drop for XContext {
    fn drop(&mut self) {
        let options = self.options.clone();

        unsafe {
            logln!(options, "Free pixmap used for background");
            XFreePixmap(self.display, self.pixmap);

            logln!(options, "Reset background to solid black and clear window");
            XSetWindowBackground(
                self.display,
                self.root,
                x11::xlib::XBlackPixel(self.display, self.screen),
            );
            XClearWindow(self.display, self.root);

            XCloseDisplay(self.display);
        }
    }
}

fn parse_window_id(
    display: *mut Display,
    root: c_ulong,
    window_id: &str,
) -> Result<c_ulong, XContextError> {
    // Check if decimal
    let decimal: Result<c_ulong, std::num::ParseIntError> = window_id.parse();
    if decimal.is_ok() {
        return Ok(decimal.unwrap());
    }

    // Else check if hexadecimal
    if window_id.starts_with("0x") {
        let decimal = c_ulong::from_str_radix(window_id.trim_start_matches("0x"), 16);
        if decimal.is_ok() {
            return Ok(decimal.unwrap());
        }
    }

    // Else ask root window for property, then check if hexadecimal or decimal
    let atom = get_atom(display, window_id.as_ptr() as *const i8, x11::xlib::True);
    if atom == x11::xlib::False as u64 {
        return Err(XContextError::with(
            EXIT_INVALID_WINDOW_ID,
            "Given window_id is neither a decimal or hexadecimal value, nor
            does an atom with name exists on root window."
                .to_string(),
        ));
    }

    match query_window_propery_as_window_id(display, root, atom) {
        Ok(prop_window_id) => Ok(prop_window_id),
        Err(e) => Err(XContextError::with(EXIT_INVALID_WINDOW_ID, e)),
    }
}

/// Parse string as X11-color.
fn parse_color(
    display: *mut Display,
    screen: c_int,
    opts: Arc<Options>,
) -> Result<XColor, XContextError> {
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

        return Err(XContextError::with(
            EXIT_UNKOWN_COLOR,
            format!(
                "Unable to parse {} as X11-color. Try hex-color format: #RRGGBB.",
                color_str
            ),
        ));
    }

    unsafe { XAllocColor(display, cmap, xcolor_ptr) };

    logln!(opts, "{:?}", xcolor);

    Ok(xcolor)
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

#[derive(Debug, Clone)]
pub struct XContextError {
    pub code: i32,
    pub message: String,
}

impl XContextError {
    fn with(code: i32, message: String) -> XContextError {
        XContextError { code, message }
    }
}

impl Error for XContextError {}

impl std::fmt::Display for XContextError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[cfg(all(test, feature = "x11-integration-tests"))]
mod tests {
    use std::ffi::CString;
    use std::sync::Arc;

    use super::Options;
    use super::XContext;
    use super::EXIT_INVALID_WINDOW_ID;
    use super::EXIT_UNKOWN_COLOR;

    use crate::position::Scaling;
    use crate::position::ScalingFilter;

    #[test]
    fn when_option_window_id_is_decimal_then_use_as_root() {
        guard_x11_test();

        let xcontext = XContext::new(create_options("4711")).unwrap();

        assert_eq!(xcontext.root, 4711);
    }

    #[test]
    fn when_option_window_id_is_hexdecimal_then_use_as_root() {
        guard_x11_test();

        let xcontext = XContext::new(create_options("0x00000FF")).unwrap();

        assert_eq!(xcontext.root, 255);
    }

    #[test]
    fn when_option_window_id_is_empty_then_use_root_window() {
        guard_x11_test();

        let xcontext = XContext::new(create_options("")).unwrap();
        let root = unsafe { x11::xlib::XRootWindow(xcontext.display, xcontext.screen) };

        assert_eq!(xcontext.root, root);
    }

    #[test]
    fn when_option_window_id_is_not_an_atom_name_then_return_error() {
        guard_x11_test();

        match XContext::new(create_options("foobar")) {
            Ok(_) => assert!(false, "Creation of XContext must fail, when arbritary string is given, but is not an atom on root."),
            Err(e) => assert_eq!(EXIT_INVALID_WINDOW_ID, e.code)
        }
    }

    #[test]
    fn when_option_window_id_is_an_atom_name_then_parse_its_value() {
        guard_x11_test();

        create_atom_window_id("test_atom", 4711);

        let xcontext = XContext::new(create_options("test_atom"));

        delete_atom("test_atom");

        match xcontext {
            Ok(xcontext) => assert_eq!(4711, xcontext.root),
            Err(_) => assert!(false),
        };
    }

    #[test]
    fn when_option_window_id_is_an_atom_name_then_parse_its_value_and_fail_if_not_an_id() {
        guard_x11_test();

        create_atom_string("foo", "bar");

        let xcontext = XContext::new(create_options("foo"));

        delete_atom("foo");

        match xcontext {
            Ok(_) => assert!(false),
            Err(e) => assert_eq!(EXIT_INVALID_WINDOW_ID, e.code),
        };
    }

    #[test]
    fn when_background_color_is_hex_rgb_then_use_as_xcolor() {
        guard_x11_test();

        let result = XContext::new(create_option_with_color("#00ffff"));

        match result {
            Ok(xcontext) => {
                assert_eq!(65535, xcontext.background_color.pixel);
                assert_eq!(0, xcontext.background_color.red);
                assert_eq!(65535, xcontext.background_color.green);
                assert_eq!(65535, xcontext.background_color.blue);
            }
            Err(_) => assert!(false),
        };
    }

    #[test]
    fn when_background_color_is_x11_color_name_then_use_as_xcolor() {
        guard_x11_test();

        let result = XContext::new(create_option_with_color("red"));

        match result {
            Ok(xcontext) => {
                assert_eq!(16711680, xcontext.background_color.pixel);
                assert_eq!(65535, xcontext.background_color.red);
                assert_eq!(0, xcontext.background_color.green);
                assert_eq!(0, xcontext.background_color.blue);
            }
            Err(_) => assert!(false),
        };
    }

    #[test]
    fn when_background_color_is_not_x11_compilant_then_fail() {
        guard_x11_test();

        let result = XContext::new(create_option_with_color("foobar"));

        match result {
            Ok(_) => assert!(false, "foobar must not result in valid background_color."),
            Err(e) => assert_eq!(EXIT_UNKOWN_COLOR, e.code),
        };
    }

    fn guard_x11_test() {
        let display = unsafe { x11::xlib::XOpenDisplay(std::ptr::null()) };

        if display.is_null() {
            assert!(false, "This test must run in a X11-session.");
        }

        unsafe { x11::xlib::XCloseDisplay(display) };
    }

    fn create_atom_window_id(name: &str, window_id: u32) {
        create_atom(
            name,
            Box::into_raw(Box::new(window_id)) as *const u8,
            x11::xlib::XA_WINDOW,
        );
    }

    fn create_atom_string(name: &str, value: &str) {
        create_atom(
            name,
            CString::new(value).unwrap().into_raw() as *const u8,
            x11::xlib::XA_STRING,
        );
    }

    fn create_atom(name: &str, data_ptr: *const u8, ptype: u64) {
        unsafe {
            let display = x11::xlib::XOpenDisplay(std::ptr::null());

            let atom = x11::xlib::XInternAtom(
                display,
                CString::new(name).unwrap().as_ptr(),
                x11::xlib::False,
            );

            let screen = x11::xlib::XDefaultScreen(display);
            let root = x11::xlib::XRootWindow(display, screen);

            x11::xlib::XChangeProperty(
                display,
                root,
                atom,
                ptype,
                32,
                x11::xlib::PropModeReplace,
                data_ptr,
                1,
            );

            x11::xlib::XFlush(display);
            x11::xlib::XCloseDisplay(display);
        }
    }

    fn delete_atom(name: &str) {
        unsafe {
            let display = x11::xlib::XOpenDisplay(std::ptr::null());

            let atom = x11::xlib::XInternAtom(
                display,
                std::ffi::CString::new(name).unwrap().as_ptr(),
                x11::xlib::False,
            );

            let screen = x11::xlib::XDefaultScreen(display);
            let root = x11::xlib::XRootWindow(display, screen);

            x11::xlib::XDeleteProperty(display, root, atom);

            x11::xlib::XCloseDisplay(display);
        }
    }

    fn create_options(window_id: &str) -> Arc<Options> {
        Arc::new(Options {
            background_color: "#000000".to_string(),
            default_delay: 100,
            path_to_gif: "foo.gif".to_string(),
            scaling: Scaling::FILL,
            scaling_filter: ScalingFilter::AUTO,
            verbose: false,
            window_id: window_id.to_string(),
        })
    }

    fn create_option_with_color(color: &str) -> Arc<Options> {
        Arc::new(Options {
            background_color: color.to_string(),
            default_delay: 100,
            path_to_gif: "foo.gif".to_string(),
            scaling: Scaling::FILL,
            scaling_filter: ScalingFilter::AUTO,
            verbose: false,
            window_id: "".to_string(),
        })
    }
}
