//! This module encapsules handling with the xshm-extenstion of the X-server.
//!
//! Main-purpose is to use shared memory between client and server for the
//! frames of the rendered GIF. This enabled a massive performance-boost,
//! compared to fast and frequent client-server-transmission of huge bitamps.

use std::ffi::c_void;
use std::mem;
use std::os::raw::{c_char, c_int};
use std::ptr::{null, null_mut};
use std::rc::Rc;

use x11::xlib::*;
use x11::xshm;

pub fn is_xshm_available() -> bool {
    let display = unsafe { XOpenDisplay(null()) };

    let status = unsafe { xshm::XShmQueryExtension(display) };

    unsafe { XCloseDisplay(display) };

    status == True
}

pub fn create_xshm_sgmnt_inf(
    data: Rc<Vec<i8>>,
    size: usize,
) -> Result<Box<xshm::XShmSegmentInfo>, u8> {
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

pub fn destroy_xshm_sgmnt_inf(seginf: &mut Box<xshm::XShmSegmentInfo>) {
    unsafe { libc::shmdt(seginf.shmaddr as *mut libc::c_void) };
}

pub fn create_xshm_image(
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
