//! This module encapsules the atom-handling with the X-server.
//!
//! There are two atoms on an X-server which hold the id of the pixmap used as
//! background for the root window. An X-compositors listens to changes on these
//! atoms and informs X-clients accordingly, so they may redraw their pseudo-
//! transparent background.

use crate::Options;
use crate::XContext;

use std::ffi::CString;

use std::os::raw::{c_char, c_int, c_uchar, c_ulong};
use std::sync::Arc;

use x11::xlib::{
    Display, False, Pixmap, PropModeReplace, True, Window, XChangeProperty, XGetWindowProperty,
    XInternAtom, XKillClient, XA_PIXMAP, XA_WINDOW,
};

const ATOM_XROOTPMAP_ID: &str = "_XROOTPMAP_ID";
const ATOM_ESETROOT_PMAP_ID: &str = "ESETROOT_PMAP_ID";

/// Convenience: Get or create atom-id with name `_XROOTPMAP_ID`.
pub fn get_root_pixmap_atom(display: *mut Display) -> c_ulong {
    get_atom(display, get_atom_name(ATOM_XROOTPMAP_ID).as_ptr(), False)
}

/// Convenience: Get or create atom-id with name `ESETROOT_PMAP_ID`.
pub fn get_eroot_pixmap_atom(display: *mut Display) -> c_ulong {
    get_atom(
        display,
        get_atom_name(ATOM_ESETROOT_PMAP_ID).as_ptr(),
        False,
    )
}

fn get_atom_name(name: &str) -> CString {
    CString::new(name).unwrap()
}

/// Gets the atom id. May create the atom on the server. If that fails or the
/// atom does not exist and should not be created, may return `xlib::False`.
pub fn get_atom(display: *mut Display, name: *const c_char, only_if_exists: c_int) -> c_ulong {
    unsafe { XInternAtom(display, name, only_if_exists) }
}

/// Convenience: Remove the pixmap related atoms on the root window.
pub fn remove_root_pixmap_atoms(xcontext: &Box<XContext>, options: Arc<Options>) -> bool {
    let mut removed_atoms = false;

    let atom_root = get_atom(
        xcontext.display,
        get_atom_name(ATOM_XROOTPMAP_ID).as_ptr(),
        True,
    );
    if atom_root != 0 {
        removed_atoms = remove_root_pixmap_atom(&xcontext, atom_root, options.clone());
    }

    let atom_eroot = get_atom(
        xcontext.display,
        get_atom_name(ATOM_ESETROOT_PMAP_ID).as_ptr(),
        True,
    );
    if atom_eroot != 0 {
        removed_atoms =
            removed_atoms || remove_root_pixmap_atom(&xcontext, atom_eroot, options.clone());
    }

    removed_atoms
}

fn remove_root_pixmap_atom(xcontext: &Box<XContext>, atom: c_ulong, options: Arc<Options>) -> bool {
    let pixmap_result = query_window_propery_as_pixmap_id(xcontext.display, xcontext.root, atom);

    if pixmap_result.is_ok() {
        let pixmap = pixmap_result.unwrap();

        if xcontext.pixmap != pixmap {
            if options.verbose {
                println!("Kill client responsible for _XROOTPMAP_ID {}", pixmap);
            }

            delete_atom(&xcontext, atom);
            unsafe { XKillClient(xcontext.display, pixmap) };
            return true;
        }
    }

    false
}

pub fn query_window_propery_as_pixmap_id(
    display: *mut Display,
    window: c_ulong,
    atom: c_ulong,
) -> Result<Pixmap, String> {
    let (_, ptype, _, _, _, data_ptr) = query_window_propery(display, window, atom, XA_PIXMAP);

    if ptype != XA_PIXMAP {
        return Err("Given atom is not a pixmap-id".to_string());
    }

    let pixmap = unsafe { Box::from_raw(data_ptr as *mut x11::xlib::Pixmap) };

    Ok(*pixmap)
}

pub fn query_window_propery_as_window_id(
    display: *mut Display,
    window: c_ulong,
    atom: c_ulong,
) -> Result<Window, String> {
    let (_, ptype, _, _, _, data_ptr) = query_window_propery(display, window, atom, XA_WINDOW);

    if ptype != XA_WINDOW {
        return Err("Given atom is not a window-id".to_string());
    }

    let window = unsafe { Box::from_raw(data_ptr as *mut Window) };

    Ok(*window)
}

/// Queries the specified atom, if existing.
/// Return tuple specifies
/// * result
/// * ptype
/// * format
/// * length
/// * pointer to data
///
/// As specified in the X-manual:
/// https://www.x.org/releases/X11R7.7/doc/man/man3/XGetWindowProperty.3.xhtml
fn query_window_propery(
    display: *mut Display,
    window: c_ulong,
    atom: c_ulong,
    property_type: c_ulong,
) -> (c_int, c_ulong, c_int, c_ulong, c_ulong, *mut c_uchar) {
    let mut ptr = std::mem::MaybeUninit::<*mut c_uchar>::uninit();

    let mut ptype = 0 as u64;
    let mut format = 0 as i32;
    let mut length = 0 as u64;
    let mut after = 0 as u64;

    let result = unsafe {
        XGetWindowProperty(
            display,
            window,
            atom,
            0,
            1,
            False,
            property_type,
            &mut ptype,
            &mut format,
            &mut length,
            &mut after,
            ptr.as_mut_ptr(),
        )
    };

    (result, ptype, format, length, after, unsafe {
        ptr.assume_init()
    })
}

pub fn delete_atom(xcontext: &Box<XContext>, atom: c_ulong) -> bool {
    unsafe { x11::xlib::XDeleteProperty(xcontext.display, xcontext.root, atom) == True }
}

pub fn update_root_pixmap_atoms(
    display: *mut Display,
    root: u64,
    pixmap_ptr: *const Pixmap,
    atom_root: c_ulong,
    atom_eroot: c_ulong,
) -> bool {
    // The pixmap itself has not changed, but its content. XChangeProperty
    // generates messages to all X-clients, to update their own rendering, if
    // needed.
    update_root_pixmap_atom(display, root, pixmap_ptr, atom_root);
    update_root_pixmap_atom(display, root, pixmap_ptr, atom_eroot);

    true
}

fn update_root_pixmap_atom(
    display: *mut Display,
    root: u64,
    pixmap_ptr: *const Pixmap,
    atom: c_ulong,
) {
    unsafe {
        XChangeProperty(
            display,
            root,
            atom,
            XA_PIXMAP,
            32,
            PropModeReplace,
            pixmap_ptr as *const u8,
            1,
        )
    };
}
