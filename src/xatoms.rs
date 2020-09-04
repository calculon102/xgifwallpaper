//! This module encapsules the atom-handling with the X-server.
//!
//! There are two atoms on an X-server which hold the id of the pixmap used as
//! background for the root window. An X-compositors listens to changes on these
//! atoms and informs X-clients accordingly, so they may redraw their pseudo-
//! transparent background.

use crate::Options;
use crate::XContext;

use std::ffi::{c_void, CString};

use std::os::raw::{c_char, c_int, c_ulong};
use std::sync::Arc;

use x11::xlib::{
    AnyPropertyType, Display, False, Pixmap, PropModeReplace, True, XChangeProperty, XFree,
    XGetWindowProperty, XInternAtom, XKillClient, XA_PIXMAP,
};

const ATOM_XROOTPMAP_ID: &str = "_XROOTPMAP_ID";
const ATOM_ESETROOT_PMAP_ID: &str = "ESETROOT_PMAP_ID";

pub fn get_root_pixmap_atom(display: *mut Display) -> c_ulong {
    get_atom(display, get_atom_name(ATOM_XROOTPMAP_ID).as_ptr(), False)
}

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

fn get_atom(display: *mut Display, name: *const c_char, only_if_exists: c_int) -> c_ulong {
    unsafe { XInternAtom(display, name, only_if_exists) }
}

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
    // Better or more declarative way to create a mutable char-pointer?
    let data = CString::new("").unwrap();
    let mut data_ptr: *mut u8 = data.as_ptr() as *mut u8;

    let mut ptype = 0 as u64;
    let mut format = 0 as i32;
    let mut length = 0 as u64;
    let mut after = 0 as u64;

    let result = unsafe {
        XGetWindowProperty(
            xcontext.display,
            xcontext.root,
            atom,
            0,
            1,
            False,
            AnyPropertyType as u64,
            &mut ptype,
            &mut format,
            &mut length,
            &mut after,
            &mut data_ptr,
        )
    };

    if result != 0 && ptype == XA_PIXMAP {
        let root_pixmap_id = unsafe { *(data_ptr as *const Pixmap) };

        if xcontext.pixmap != root_pixmap_id {
            if options.verbose {
                println!(
                    "Kill client responsible for _XROOTPMAP_ID {}",
                    root_pixmap_id
                );
            }

            delete_atom(&xcontext, atom);
            unsafe { XKillClient(xcontext.display, root_pixmap_id) };
            unsafe { XFree(data_ptr as *mut c_void) };
            return true;
        }
    }

    false
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
