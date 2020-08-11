use std::fmt;
use std::os::raw::c_int;
use std::ptr;

use x11::xinerama;
use x11::xlib;

pub struct ScreenInfo {
    root_per_screen: bool,
    screens: Vec<Screen>,
}

impl fmt::Debug for ScreenInfo {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("ScreenInfo")
            .field("root_per_screen", &self.root_per_screen)
            .field("screens", &self.screens)
            .finish()
    }
}

pub struct Screen {
    screen_number: i32,
    x_org: i32,
    y_org: i32,
    width: i32,
    height: i32,
}

impl fmt::Debug for Screen {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("Screen")
            .field("screen_number", &self.screen_number)
            .field("x_org", &self.x_org)
            .field("y_org", &self.y_org)
            .field("width", &self.width)
            .field("height", &self.height)
            .finish()
    }
}

pub fn get_screen_info() -> ScreenInfo {
    let mut root_per_screen = false;
    let mut screens: Vec<Screen> = Vec::new();

    unsafe {
        let display = xlib::XOpenDisplay(ptr::null());

        if use_xinerama(display) {
            let mut screen_count = 0;
            let xscreens = xinerama::XineramaQueryScreens(display, &mut screen_count);

            for i in 0..(screen_count) {
                screens.push(Screen {
                    screen_number: (*xscreens.offset(i as isize)).screen_number,
                    x_org: (*xscreens.offset(i as isize)).x_org as i32,
                    y_org: (*xscreens.offset(i as isize)).y_org as i32,
                    width: (*xscreens.offset(i as isize)).width as i32,
                    height: (*xscreens.offset(i as isize)).height as i32,
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
                    width: xlib::XDisplayWidth(display, i),
                    height: xlib::XDisplayHeight(display, i),
                });
            }
        }

        xlib::XCloseDisplay(display);
    }

    return ScreenInfo {
        root_per_screen: root_per_screen,
        screens: screens,
    };
}

unsafe fn use_xinerama(display: *mut xlib::Display) -> bool {
    let mut event_base_return: c_int = 0;
    let mut error_base_return: c_int = 0;

    let has_extension =
        xinerama::XineramaQueryExtension(display, &mut event_base_return, &mut error_base_return);

    let is_active = xinerama::XineramaIsActive(display);

    return has_extension == xlib::True && is_active == xlib::True;
}
