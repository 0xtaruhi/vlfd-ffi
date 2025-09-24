//! C FFI for vlfd-rs (minimal API)
use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_uint, c_void};

use vlfd_rs::{Device, IoSettings, Programmer};

#[repr(C)]
pub struct VlfdDevice {
    inner: *mut c_void,
}

thread_local! {
    static LAST_ERROR: RefCell<Option<CString>> = const { RefCell::new(None) };
}

fn set_last_error(message: &str) {
    let cstr =
        CString::new(message).unwrap_or_else(|_| CString::new("<invalid utf8 in error>").unwrap());
    LAST_ERROR.with(|slot| {
        *slot.borrow_mut() = Some(cstr);
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn vlfd_get_last_error_message() -> *const c_char {
    static EMPTY: &[u8] = b"\0";
    LAST_ERROR.with(|slot| match &*slot.borrow() {
        Some(s) => s.as_ptr(),
        None => EMPTY.as_ptr() as *const c_char,
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn vlfd_io_open() -> *mut VlfdDevice {
    let mut dev = match Device::connect() {
        Ok(d) => d,
        Err(e) => {
            set_last_error(&format!("connect failed: {}", e));
            return std::ptr::null_mut();
        }
    };

    if let Err(e) = dev.enter_io_mode(&IoSettings::default()) {
        set_last_error(&format!("enter_io_mode failed: {}", e));
        return std::ptr::null_mut();
    }

    Box::into_raw(Box::new(VlfdDevice {
        inner: Box::into_raw(Box::new(dev)) as *mut c_void,
    }))
}

#[unsafe(no_mangle)]
pub extern "C" fn vlfd_io_write_read(
    device: *mut VlfdDevice,
    write_buffer: *mut u16,
    read_buffer: *mut u16,
    word_len: c_uint,
) -> c_int {
    if device.is_null() || write_buffer.is_null() || read_buffer.is_null() {
        set_last_error("null pointer passed to vlfd_io_write_read");
        return -1;
    }

    let dev = unsafe { &mut *((*device).inner as *mut Device) };
    let len = word_len as usize;
    let tx = unsafe { std::slice::from_raw_parts_mut(write_buffer, len) };
    let rx = unsafe { std::slice::from_raw_parts_mut(read_buffer, len) };

    match dev.transfer_io(tx, rx) {
        Ok(_) => 0,
        Err(e) => {
            set_last_error(&format!("transfer_io failed: {}", e));
            -1
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vlfd_io_close(device: *mut VlfdDevice) -> c_int {
    if device.is_null() {
        set_last_error("null device in vlfd_io_close");
        return -1;
    }

    unsafe {
        let wrapper = Box::from_raw(device);
        let dev_ptr = wrapper.inner as *mut Device;
        if dev_ptr.is_null() {
            set_last_error("invalid device handle");
            return -1;
        }
        let mut dev = Box::from_raw(dev_ptr);

        if let Err(e) = dev.exit_io_mode() {
            set_last_error(&format!("exit_io_mode failed: {}", e));
        }
        if let Err(e) = dev.close() {
            set_last_error(&format!("close failed: {}", e));
        }
        // Boxes dropped here free both wrapper and device
    }

    0
}

#[unsafe(no_mangle)]
pub extern "C" fn vlfd_program_fpga(bitfile_path: *const c_char) -> c_int {
    if bitfile_path.is_null() {
        set_last_error("null bitfile_path");
        return -1;
    }

    let path_cstr = unsafe { CStr::from_ptr(bitfile_path) };
    let path = match path_cstr.to_str() {
        Ok(s) => std::path::Path::new(s),
        Err(_) => {
            set_last_error("bitfile_path is not valid UTF-8");
            return -1;
        }
    };

    let mut prog = match Programmer::connect() {
        Ok(p) => p,
        Err(e) => {
            set_last_error(&format!("programmer connect failed: {}", e));
            return -1;
        }
    };

    if let Err(e) = prog.program(path) {
        let _ = prog.close();
        set_last_error(&format!("program failed: {}", e));
        return -1;
    }

    match prog.close() {
        Ok(_) => 0,
        Err(e) => {
            set_last_error(&format!("programmer close failed: {}", e));
            -1
        }
    }
}
