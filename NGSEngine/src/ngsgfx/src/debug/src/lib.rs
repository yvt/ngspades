//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! NgsGFX Debug Tools
//! ==================
//!
//! Provides debug tools for NgsGFX.
//!
//! This crate is re-exported by the `ngsgfx` main crate as `::debug`.
//!
extern crate ngsgfx_core as core;
extern crate term;
extern crate chrono;

use std::sync::Mutex;
use std::io;

/// The debug report handler that outputs messages using `print`.
///
/// This handler does not support styling, but is the only handler whose output
/// can be captured during `cargo test`.
pub struct PrintDebugReportHandler(Mutex<()>);

impl PrintDebugReportHandler {
    pub fn new() -> Self {
        PrintDebugReportHandler(Mutex::new(()))
    }
}

impl core::DebugReportHandler for PrintDebugReportHandler {
    fn log(&self, report: &core::DebugReport) {
        use chrono::prelude::*;
        use core::DebugReportType;

        let _ = self.0.lock().unwrap();

        let dt = Local::now();
        print!("{} ", dt.format("%Y-%m-%d %H:%M:%S"));
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

/// The debug report handler that outputs messages using `std::io::Write`.
pub struct WriteDebugReportHandler<T> {
    t: Mutex<T>,
}

/// The debug report handler that outputs messages via `std::io::stdout()`.
pub type StdoutDebugReportHandler = WriteDebugReportHandler<io::Stdout>;

/// The debug report handler that outputs messages via `std::io::stdout()`.
pub type StderrDebugReportHandler = WriteDebugReportHandler<io::Stderr>;

impl<T> WriteDebugReportHandler<T> {
    pub fn from_terminal(t: T) -> Self {
        Self { t: Mutex::new(t) }
    }
}

impl StdoutDebugReportHandler {
    pub fn new() -> Self {
        Self::from_terminal(io::stdout())
    }
}

impl StderrDebugReportHandler {
    pub fn new() -> Self {
        Self::from_terminal(io::stderr())
    }
}

impl<T: io::Write + Send> core::DebugReportHandler for WriteDebugReportHandler<T> {
    fn log(&self, report: &core::DebugReport) {
        use chrono::prelude::*;
        use core::DebugReportType;

        let mut t = self.t.lock().unwrap();
        let dt = Local::now();
        write!(t, "{} ", dt.format("%Y-%m-%d %H:%M:%S")).unwrap();
        match report.typ {
            DebugReportType::Debug => {
                write!(t, "DEBUG ").unwrap();
            }
            DebugReportType::Information => {
                write!(t, "INFO  ").unwrap();
            }
            DebugReportType::Warning => {
                write!(t, "WARN  ").unwrap();
            }
            DebugReportType::PerformanceWarning => {
                write!(t, "PERF  ").unwrap();
            }
            DebugReportType::Error => {
                write!(t, "ERROR ").unwrap();
            }
        }
        writeln!(t, "{}", report.message).unwrap();
    }
}

/// The debug report handler that outputs messages to a terminal.
///
/// This function uses the [`term`] crate to colorize the output.
///
/// [`term`]: https://crates.io/crates/term
pub struct TermDebugReportHandler<T: ?Sized> {
    t: Mutex<Option<Box<T>>>,
}

/// The debug report handler that outputs messages to the stdout via the `term` crate.
///
///
///
/// [`term`]: https://crates.io/crates/term
pub type TermStdoutDebugReportHandler = TermDebugReportHandler<term::StdoutTerminal>;

/// The debug report handler that outputs messages to the stderr via the `term` crate.
///
/// [`term`]: https://crates.io/crates/term
pub type TermStderrDebugReportHandler = TermDebugReportHandler<term::StderrTerminal>;

impl<T: ?Sized> TermDebugReportHandler<T> {
    pub fn from_terminal(t: Option<Box<T>>) -> Self {
        Self { t: Mutex::new(t) }
    }
}

impl TermStdoutDebugReportHandler {
    pub fn new() -> Self {
        Self::from_terminal(term::stdout())
    }
}

impl TermStderrDebugReportHandler {
    pub fn new() -> Self {
        Self::from_terminal(term::stderr())
    }
}

impl<T: term::Terminal + Send + ?Sized> core::DebugReportHandler for TermDebugReportHandler<T> {
    fn log(&self, report: &core::DebugReport) {
        use chrono::prelude::*;
        use core::DebugReportType;

        if let Some(mut t) = self.t.lock().unwrap().as_mut() {
            let dt = Local::now();
            write!(t, "{} ", dt.format("%Y-%m-%d %H:%M:%S")).unwrap();
            match report.typ {
                DebugReportType::Debug => {
                    t.fg(term::color::WHITE).unwrap();
                    write!(t, "DEBUG ").unwrap();
                }
                DebugReportType::Information => {
                    t.fg(term::color::BRIGHT_CYAN).unwrap();
                    write!(t, "INFO  ").unwrap();
                }
                DebugReportType::Warning => {
                    t.fg(term::color::BRIGHT_YELLOW).unwrap();
                    write!(t, "WARN  ").unwrap();
                }
                DebugReportType::PerformanceWarning => {
                    t.fg(term::color::BRIGHT_GREEN).unwrap();
                    write!(t, "PERF  ").unwrap();
                }
                DebugReportType::Error => {
                    t.fg(term::color::BRIGHT_RED).unwrap();
                    write!(t, "ERROR ").unwrap();
                }
            }
            t.reset().unwrap();
            writeln!(t, "{}", report.message).unwrap();
        }
    }
}
