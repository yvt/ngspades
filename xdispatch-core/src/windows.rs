use std::os::raw::c_ulong;

/// from `libkqueue/src/windows/platform.c`
#[repr(C)]
#[derive(Default)]
struct event_buf {
    bytes: c_ulong,
    key: usize,
    overlap: usize,
}

// Making a TLV with destructor and getting it working across various
// binary types (e.g., exe, dll, ...) is very tricky.
// Fortunately, Rust's libstd provides a very reliable implementation.
thread_local! {
	static IOCP_BUF: event_buf = Default::default();
}

#[no_mangle]
pub extern "C" fn libkqueue_iocp_buf() -> *mut () {
	IOCP_BUF.with(|r| r as *const _ as *mut event_buf as *mut ())
}
