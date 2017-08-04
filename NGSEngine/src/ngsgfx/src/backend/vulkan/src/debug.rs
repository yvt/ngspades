//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use ash::vk;
use ash::version::EntryV1_0;
use ash::extensions::DebugReport;
use std::sync::Arc;
use std::{ptr, fmt};
use core;

use {InstanceRef, AshInstance, translate_generic_error_unwrap};

/// Wraps the interface to the `VK_EXT_debug_report` instance extension.
pub struct DebugReportConduit<T: InstanceRef> {
	instance_ref: T,
	ext: DebugReport,
	callbacks: Vec<DebugReportCallback>,
}

impl<T: InstanceRef> fmt::Debug for DebugReportConduit<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("DebugReportConduit")
            .field("instance_ref", &self.instance_ref)
            .field("ext", &())
            .field("callbacks", &())
            .finish()
    }
}

struct DebugReportCallback {
	handle: vk::DebugReportCallbackEXT,
	data: Box<DebugReportCallbackData>,
}

struct DebugReportCallbackData(Arc<core::DebugReportHandler>, core::DebugReportType);

impl<T: InstanceRef> Drop for DebugReportConduit<T> {
	fn drop(&mut self) {
		for callback in self.callbacks.drain(..) {
			unsafe {
				self.ext.destroy_debug_report_callback_ext(callback.handle, self.instance_ref.allocation_callbacks());
			}
		}
	}
}

unsafe extern "system" fn debug_callback(
	_: vk::DebugReportFlagsEXT,
    _: vk::DebugReportObjectTypeEXT,
    _: vk::uint64_t,
    _: vk::size_t,
    _: vk::int32_t,
    p_layer_prefix: *const vk::c_char,
    p_message: *const vk::c_char,
    p_user_data: *mut vk::c_void
) -> u32 {
	use std::ffi::CStr;

	let layer_prefix = CStr::from_ptr(p_layer_prefix).to_string_lossy();
	let message = CStr::from_ptr(p_message).to_string_lossy();
	let ref data: DebugReportCallbackData = *(p_user_data as *const _);

	data.0.log(&core::DebugReport{
		typ: data.1,
		message: &format!("{}: {}", layer_prefix, message),
	});

    vk::VK_TRUE
}

impl<T: InstanceRef> DebugReportConduit<T> {
	pub fn new<E: EntryV1_0>(entry: &E, instance_ref: &T) -> Result<Self, Vec<&'static str>> {
		Ok(Self{
			instance_ref: instance_ref.clone(),
			ext: DebugReport::new(entry, instance_ref.instance())?,
			callbacks: Vec::new(),
		})
	}

	pub fn add_handler(&mut self, flags: core::DebugReportTypeFlags, handler: Arc<core::DebugReportHandler>) {
		for &(typ, vk_typ) in [
			(core::DebugReportType::Information, vk::DEBUG_REPORT_INFORMATION_BIT_EXT),
			(core::DebugReportType::Warning, vk::DEBUG_REPORT_WARNING_BIT_EXT),
			(core::DebugReportType::PerformanceWarning, vk::DEBUG_REPORT_PERFORMANCE_WARNING_BIT_EXT),
			(core::DebugReportType::Error, vk::DEBUG_REPORT_ERROR_BIT_EXT),
			(core::DebugReportType::Debug, vk::DEBUG_REPORT_DEBUG_BIT_EXT),
		].iter() {
			if flags.contains(typ) {
				self.callbacks.reserve(1);
				let mut data = Box::new(DebugReportCallbackData(Arc::clone(&handler), typ));
				let handle = unsafe {
					self.ext.create_debug_report_callback_ext(&vk::DebugReportCallbackCreateInfoEXT{
						s_type: vk::StructureType::DebugReportCallbackCreateInfoExt,
						p_next: ptr::null(),
						flags: vk_typ,
						pfn_callback: debug_callback,
						p_user_data: &mut*data as *mut DebugReportCallbackData as *mut _,
					}, self.instance_ref.allocation_callbacks())
				}.map_err(translate_generic_error_unwrap).unwrap();
				self.callbacks.push(DebugReportCallback{ handle, data });
			}
		}
	}
}
