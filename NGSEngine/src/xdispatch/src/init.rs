// Platform-specific initializer for libxdispatch.
#[cfg(windows)]
mod win32 {
	extern "system" {
		fn xdispatch_tls_callback();
	}
	#[link_section=".CRT$XLB"]
	#[used]
	#[allow(non_upper_case_globals)]
	static xdispatch_tls_callback_func: unsafe extern "system" fn() = xdispatch_tls_callback;
}
