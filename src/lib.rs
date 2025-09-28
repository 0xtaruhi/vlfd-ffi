//! C FFI for vlfd-rs (minimal API)
use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_uint, c_void};

use vlfd_rs::{
    Device,
    HotplugEvent,
    HotplugEventKind,
    HotplugOptions,
    HotplugRegistration,
    IoSettings,
    Programmer,
};

#[repr(C)]
pub struct VlfdDevice {
    inner: *mut c_void,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct VlfdHotplugOptions {
    pub filter_vendor_id: bool,
    pub vendor_id: u16,
    pub filter_product_id: bool,
    pub product_id: u16,
    pub filter_class_code: bool,
    pub class_code: u8,
    pub enumerate_existing: bool,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct VlfdOptionalU16 {
    pub has_value: bool,
    pub value: u16,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct VlfdOptionalU8 {
    pub has_value: bool,
    pub value: u8,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct VlfdSliceU8 {
    pub data: *const u8,
    pub len: usize,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VlfdHotplugEventKind {
    Arrived = 0,
    Left = 1,
}

impl From<HotplugEventKind> for VlfdHotplugEventKind {
    fn from(value: HotplugEventKind) -> Self {
        match value {
            HotplugEventKind::Arrived => VlfdHotplugEventKind::Arrived,
            HotplugEventKind::Left => VlfdHotplugEventKind::Left,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct VlfdHotplugDeviceInfo {
    pub bus_number: u8,
    pub address: u8,
    pub port_numbers: VlfdSliceU8,
    pub vendor_id: VlfdOptionalU16,
    pub product_id: VlfdOptionalU16,
    pub class_code: VlfdOptionalU8,
    pub sub_class_code: VlfdOptionalU8,
    pub protocol_code: VlfdOptionalU8,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VlfdHotplugEvent {
    pub kind: VlfdHotplugEventKind,
    pub device: VlfdHotplugDeviceInfo,
}

#[repr(C)]
pub struct VlfdHotplugRegistration {
    inner: *mut c_void,
}

pub type VlfdHotplugCallback =
    Option<unsafe extern "C" fn(user_data: *mut c_void, event: *const VlfdHotplugEvent)>;

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

fn opt_u16(value: Option<u16>) -> VlfdOptionalU16 {
    match value {
        Some(v) => VlfdOptionalU16 {
            has_value: true,
            value: v,
        },
        None => VlfdOptionalU16 {
            has_value: false,
            value: 0,
        },
    }
}

fn opt_u8(value: Option<u8>) -> VlfdOptionalU8 {
    match value {
        Some(v) => VlfdOptionalU8 {
            has_value: true,
            value: v,
        },
        None => VlfdOptionalU8 {
            has_value: false,
            value: 0,
        },
    }
}

fn hotplug_options_from_ffi(options: Option<&VlfdHotplugOptions>) -> HotplugOptions {
    let mut result = HotplugOptions::default();
    if let Some(opts) = options {
        if opts.filter_vendor_id {
            result.vendor_id = Some(opts.vendor_id);
        }
        if opts.filter_product_id {
            result.product_id = Some(opts.product_id);
        }
        if opts.filter_class_code {
            result.class_code = Some(opts.class_code);
        }
        result.enumerate = opts.enumerate_existing;
    }
    result
}

fn hotplug_event_to_ffi(event: HotplugEvent) -> (VlfdHotplugEvent, Vec<u8>) {
    let kind = event.kind;
    let device = event.device;

    let bus_number = device.bus_number;
    let address = device.address;
    let vendor_id = device.vendor_id;
    let product_id = device.product_id;
    let class_code = device.class_code;
    let sub_class_code = device.sub_class_code;
    let protocol_code = device.protocol_code;
    let port_numbers = device.port_numbers;

    let info = VlfdHotplugDeviceInfo {
        bus_number,
        address,
        port_numbers: VlfdSliceU8 {
            data: port_numbers.as_ptr(),
            len: port_numbers.len(),
        },
        vendor_id: opt_u16(vendor_id),
        product_id: opt_u16(product_id),
        class_code: opt_u8(class_code),
        sub_class_code: opt_u8(sub_class_code),
        protocol_code: opt_u8(protocol_code),
    };

    (
        VlfdHotplugEvent {
            kind: kind.into(),
            device: info,
        },
        port_numbers,
    )
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
pub extern "C" fn vlfd_hotplug_options_default() -> VlfdHotplugOptions {
    VlfdHotplugOptions::default()
}

#[unsafe(no_mangle)]
pub extern "C" fn vlfd_hotplug_register(
    options: *const VlfdHotplugOptions,
    callback: VlfdHotplugCallback,
    user_data: *mut c_void,
) -> *mut VlfdHotplugRegistration {
    let callback = match callback {
        Some(cb) => cb,
        None => {
            set_last_error("null callback passed to vlfd_hotplug_register");
            return std::ptr::null_mut();
        }
    };

    let options_ref = unsafe { options.as_ref() };
    let rust_options = hotplug_options_from_ffi(options_ref);
    let user_data_value = user_data as usize;

    let device = match Device::new() {
        Ok(dev) => dev,
        Err(err) => {
            set_last_error(&format!("Device::new failed: {}", err));
            return std::ptr::null_mut();
        }
    };

    let registration = match device.usb().register_hotplug_callback(rust_options, move |event| {
        let (ffi_event, ports) = hotplug_event_to_ffi(event);
        unsafe {
            let userdata_ptr = user_data_value as *mut c_void;
            callback(userdata_ptr, &ffi_event as *const VlfdHotplugEvent);
        }
        drop(ports);
    }) {
        Ok(reg) => reg,
        Err(err) => {
            set_last_error(&format!("register_hotplug_callback failed: {}", err));
            return std::ptr::null_mut();
        }
    };

    let boxed_registration = Box::new(registration);
    let handle = Box::new(VlfdHotplugRegistration {
        inner: Box::into_raw(boxed_registration) as *mut c_void,
    });

    Box::into_raw(handle)
}

#[unsafe(no_mangle)]
pub extern "C" fn vlfd_hotplug_unregister(registration: *mut VlfdHotplugRegistration) -> c_int {
    if registration.is_null() {
        set_last_error("null registration in vlfd_hotplug_unregister");
        return -1;
    }

    unsafe {
        let wrapper = Box::from_raw(registration);
        if wrapper.inner.is_null() {
            set_last_error("invalid registration handle");
            return -1;
        }
        let inner = Box::from_raw(wrapper.inner as *mut HotplugRegistration);
        drop(inner);
    }

    0
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
