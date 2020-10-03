//! Query `Screens` and define `Screen`-structure.

use std::os::raw::c_int;
use std::ptr;

use x11::xinerama;
use x11::xlib;

/// Collection of screens.
#[derive(Debug)]
pub struct Screens {
    /// `true`, if every screen has its own root-window.
    pub root_per_screen: bool,
    /// Ordered list of `Screen`.
    pub screens: Vec<Screen>,
}

/// Information about a single screen.
#[derive(Clone, Debug)]
pub struct Screen {
    /// Logical number / id of screen.
    pub screen_number: i32,
    /// Origin on x-axis of this screen in a combined display-arrangement.
    pub x_org: i32,
    /// Origin on y-axis of this screen in a combined display-arrangement.
    pub y_org: i32,
    /// Width of this screen.
    pub width: u32,
    /// Height of this screen.
    pub height: u32,
}

impl Screens {
    /// Queries the running x-server for available screens.
    pub fn query_x_screens() -> Screens {
        let mut root_per_screen = false;
        let mut screens: Vec<Screen> = Vec::new();

        unsafe {
            let display = xlib::XOpenDisplay(ptr::null());

            if Screens::use_xinerama(display) {
                let mut screen_count = 0;
                let xscreens = xinerama::XineramaQueryScreens(display, &mut screen_count);

                for i in 0..(screen_count) {
                    screens.push(Screen {
                        screen_number: (*xscreens.offset(i as isize)).screen_number,
                        x_org: (*xscreens.offset(i as isize)).x_org as i32,
                        y_org: (*xscreens.offset(i as isize)).y_org as i32,
                        width: (*xscreens.offset(i as isize)).width as u32,
                        height: (*xscreens.offset(i as isize)).height as u32,
                    });
                }
            } else {
                root_per_screen = true;

                let screen_count = xlib::XScreenCount(display);

                for i in 0..(screen_count) {
                    screens.push(Screen {
                        screen_number: i,
                        x_org: 0,
                        y_org: 0,
                        width: xlib::XDisplayWidth(display, i) as u32,
                        height: xlib::XDisplayHeight(display, i) as u32,
                    });
                }
            }

            xlib::XCloseDisplay(display);
        }

        Screens {
            root_per_screen,
            screens,
        }
    }

    /// `true` if current x-session supports Xinerama
    unsafe fn use_xinerama(display: *mut xlib::Display) -> bool {
        let mut event_base_return: c_int = 0;
        let mut error_base_return: c_int = 0;

        let has_extension = xinerama::XineramaQueryExtension(
            display,
            &mut event_base_return,
            &mut error_base_return,
        );

        let is_active = xinerama::XineramaIsActive(display);

        return has_extension == xlib::True && is_active == xlib::True;
    }
}
