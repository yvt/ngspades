// Platform-specific initializer for libxdispatch.
#[cfg(windows)]
#[doc(hidden)]
pub mod win32 {
	extern "system" {
		fn xdispatch_tls_callback();
	}

	// The sections `.CRT$XLA` through `.CRT$$XLZ` are defined by CRT to allow
	// a program to register a set of TLS callbacks. The operating system calls
	// each callback whenever a thread starts or ends in the current process.
	// FIXME: Get this working on doctests
	#[link_section=".CRT$XLB"]
	#[used]
	#[allow(non_upper_case_globals)]
	pub static xdispatch_tls_callback_func: unsafe extern "system" fn() = xdispatch_tls_callback;
}
