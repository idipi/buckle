//! Native FFI bindings to Apple's libSystem.B.dylib sandbox subsystem.
//! Applies kernel-level Mandatory Access Control (Seatbelt) in-memory.

use anyhow::{bail, Result};
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::ptr;

extern "C" {
    fn sandbox_init(
        profile: *const c_char,
        flags: u64,
        errorbuf: *mut *mut c_char,
    ) -> c_int;
    fn sandbox_free_error(errorbuf: *mut c_char);
}

/// Apply a compiled Apple Seatbelt profile directly in memory.
///
/// Bypasses the deprecated `/usr/bin/sandbox-exec` utility and avoids writing
/// any `.sb` profile files to disk.
pub fn apply_sandbox_profile(profile_str: &str) -> Result<()> {
    let c_profile = CString::new(profile_str)?;
    let mut err_buf: *mut c_char = ptr::null_mut();

    let ret = unsafe { sandbox_init(c_profile.as_ptr(), 0, &mut err_buf) };
    if ret != 0 {
        let err_msg = if !err_buf.is_null() {
            let msg = unsafe { CStr::from_ptr(err_buf).to_string_lossy().into_owned() };
            unsafe { sandbox_free_error(err_buf) };
            msg
        } else {
            "Unknown kernel sandbox initialization error".to_string()
        };
        bail!("Kernel sandbox initialization failed: {}", err_msg);
    }
    Ok(())
}
