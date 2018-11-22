# XDispatch for Rust

This repository provides a set of crates for using [XDispatch] from a Rust program.

[XDispatch]: http://opensource.mlba-team.de/xdispatch/docs/current/index.html

> XDispatch provides the power of Grand Central Dispatch not only on Mac OS 10.6+ but also on Windows and Linux.

The following platforms are supported: Windows, Linux, and macOS.

## Crates

`xdispatch` is a fork of Steven Sheldon's [`dispatch`] (Rust wrapper for Apple's GCD) including minor modifications for a statically-linked version of XDispatch. The raw FFI interface was moved to `xdispatch-core`.

[`dispatch`]: http://github.com/SSheldon/rust-dispatch

`xdispatch-core` provides a raw FFI interface to XDispatch as well as XDispatch itself. It links the system library when GCD is available natively. In other cases (i.e., on a non-macOS/iOS target), it builds the packaged version of XDispatch and links it statically.

## License/Credits

See each crate's `Cargo.toml`.
