//! Encapsule handling with the xshm-extension of the X-server.
//!
//! The shared memory is used to avoid expensive transfer of frame-images
//! between client and server. Thus bringing a significant performance-boost.

use std::mem;
use std::os::raw::{c_char, c_int};
use std::ptr::{null, null_mut};

use x11::xlib::*;
use x11::xshm;

/// Returns `true` if X-Server supports xshm.
pub fn is_xshm_available(display: *mut Display) -> bool {
    let status = unsafe { xshm::XShmQueryExtension(display) };

    unsafe { XCloseDisplay(display) };

    status == True
}

/// Creates info-structure for the shared-memory-segment. This structure must
/// exist as long as the segment and data itself.
pub fn create_xshm_sgmnt_inf(size: usize) -> Result<Box<xshm::XShmSegmentInfo>, u8> {
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

    Ok(Box::new(xshm::XShmSegmentInfo {
        shmseg: 0,
        shmid,
        shmaddr: (shmaddr as *mut c_char),
        readOnly: 0,
    }))
}

/// Destroys the given info-structure and frees its memory.
pub fn destroy_xshm_sgmnt_inf(seginf: &mut Box<xshm::XShmSegmentInfo>) {
    unsafe { libc::shmdt(seginf.shmaddr as *mut libc::c_void) };
}

/// Creates a new `XImage`-instance, representing the data of the shared memory
/// segment.
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
        Ok(ximg)
    }
}
