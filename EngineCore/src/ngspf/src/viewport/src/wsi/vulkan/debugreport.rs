//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use super::ash::{self, extensions, vk};
use bitflags::bitflags;
use std::sync::Arc;
use std::{fmt, ptr};

use super::utils::translate_generic_error_unwrap;

/// Debug report provided by validation layers.
///
/// This was formerly a part of NgsGFX Core.
#[derive(Debug, Clone)]
pub struct DebugReport<'a> {
    pub typ: DebugReportType,
    pub message: &'a str,
}

/// Receives `DebugReport`s generated by drivers and validation layers.
pub trait DebugReportHandler: Send + Sync {
    fn log(&self, report: &DebugReport);
}

bitflags! {
    pub struct DebugReportTypeFlags: u32 {
        /// Informational messages that may be handy when debugging an
        /// application.
        const Information = 0b00001;

        /// Reports for potentially wrong, but not immediately harmful API usages.
        const Warning = 0b00010;

        /// Reports for non-optimal API usages.
        const PerformanceWarning = 0b00100;

        /// Reports for usages that may cause undefined results.
        const Error = 0b01000;

        /// Diagnostic informations.
        const Debug = 0b10000;
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum DebugReportType {
    /// Informational messages that may be handy when debugging an
    /// application.
    Information,

    /// Reports for potentially wrong, but not immediately harmful API usages.
    Warning,

    /// Reports for non-optimal API usages.
    PerformanceWarning,

    /// Reports for usages that may cause undefined results.
    Error,

    /// Diagnostic informations.
    Debug,
}

/// Wraps the interface to the `VK_EXT_debug_report` instance extension.
pub struct DebugReportConduit {
    ext: extensions::ext::DebugReport,
    callbacks: Vec<DebugReportCallback>,
}

impl fmt::Debug for DebugReportConduit {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("DebugReportConduit")
            .field("ext", &())
            .field("callbacks", &())
            .finish()
    }
}

struct DebugReportCallback {
    handle: vk::DebugReportCallbackEXT,
    data: Box<DebugReportCallbackData>,
}

struct DebugReportCallbackData(Arc<DebugReportHandler>, DebugReportType);

impl Drop for DebugReportConduit {
    fn drop(&mut self) {
        for callback in self.callbacks.drain(..) {
            unsafe {
                self.ext
                    .destroy_debug_report_callback(callback.handle, None);
            }
        }
    }
}

unsafe extern "system" fn debug_callback(
    _: vk::DebugReportFlagsEXT,
    _: vk::DebugReportObjectTypeEXT,
    _: u64,
    _: usize,
    _: i32,
    p_layer_prefix: *const std::os::raw::c_char,
    p_message: *const std::os::raw::c_char,
    p_user_data: *mut std::os::raw::c_void,
) -> u32 {
    use std::ffi::CStr;

    let layer_prefix = CStr::from_ptr(p_layer_prefix).to_string_lossy();
    let message = CStr::from_ptr(p_message).to_string_lossy();
    let ref data: DebugReportCallbackData = *(p_user_data as *const _);

    data.0.log(&DebugReport {
        typ: data.1,
        message: &format!("{}: {}", layer_prefix, message),
    });

    vk::TRUE
}

impl DebugReportConduit {
    pub fn new(entry: &ash::Entry, instance: &ash::Instance) -> Self {
        Self {
            ext: extensions::ext::DebugReport::new(entry, instance),
            callbacks: Vec::new(),
        }
    }

    pub fn add_handler(&mut self, flags: DebugReportTypeFlags, handler: Arc<DebugReportHandler>) {
        for &(typ_flag, vk_typ, typ) in [
            (
                DebugReportTypeFlags::Information,
                vk::DebugReportFlagsEXT::INFORMATION,
                DebugReportType::Information,
            ),
            (
                DebugReportTypeFlags::Warning,
                vk::DebugReportFlagsEXT::WARNING,
                DebugReportType::Warning,
            ),
            (
                DebugReportTypeFlags::PerformanceWarning,
                vk::DebugReportFlagsEXT::PERFORMANCE_WARNING,
                DebugReportType::PerformanceWarning,
            ),
            (
                DebugReportTypeFlags::Error,
                vk::DebugReportFlagsEXT::ERROR,
                DebugReportType::Error,
            ),
            (
                DebugReportTypeFlags::Debug,
                vk::DebugReportFlagsEXT::DEBUG,
                DebugReportType::Debug,
            ),
        ]
        .iter()
        {
            if flags.contains(typ_flag) {
                self.callbacks.reserve(1);
                let mut data = Box::new(DebugReportCallbackData(Arc::clone(&handler), typ));
                let handle = unsafe {
                    self.ext.create_debug_report_callback(
                        &vk::DebugReportCallbackCreateInfoEXT {
                            s_type: vk::StructureType::DEBUG_REPORT_CALLBACK_CREATE_INFO_EXT,
                            p_next: ptr::null(),
                            flags: vk_typ,
                            pfn_callback: Some(debug_callback),
                            p_user_data: &mut *data as *mut DebugReportCallbackData as *mut _,
                        },
                        None,
                    )
                }
                .map_err(translate_generic_error_unwrap)
                .unwrap();
                self.callbacks.push(DebugReportCallback { handle, data });
            }
        }
    }
}

use std::sync::Mutex;

/// The debug report handler that outputs messages using `print`.
pub struct PrintDebugReportHandler(Mutex<()>);

impl PrintDebugReportHandler {
    pub fn new() -> Self {
        PrintDebugReportHandler(Mutex::new(()))
    }
}

impl DebugReportHandler for PrintDebugReportHandler {
    fn log(&self, report: &DebugReport) {
        let _ = self.0.lock().unwrap();
        match report.typ {
            DebugReportType::Debug => {
                print!("DEBUG ");
            }
            DebugReportType::Information => {
                print!("INFO  ");
            }
            DebugReportType::Warning => {
                print!("WARN  ");
            }
            DebugReportType::PerformanceWarning => {
                print!("PERF  ");
            }
            DebugReportType::Error => {
                print!("ERROR ");
            }
        }
        println!("{}", report.message);
    }
}
