//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use std::fmt::Debug;

use super::DeviceCapabilities;

/// Trait for description types whose usage can be validated.
///
/// Unique restrictions by the backend that cannot represented in `DeviceLimits` and `DeviceCapabilities`
/// are not validated.
pub trait Validate: Debug {
    type Error: Debug;

    /// Validate the value specified via `self`.
    /// `DeviceCapabilities` can be omit in which case validation related to hardware capabilites is not performed.
    ///
    /// `callback` is called for every errors reported.
    fn validate<T>(&self, cap: Option<&DeviceCapabilities>, callback: T)
        where T: FnMut(Self::Error) -> ();

    /// Return whether values in `self` are valid.
    fn is_valid(&self, cap: Option<&DeviceCapabilities>) -> bool {
        let mut valid = true;
        self.validate(cap, |_| { valid = false; });
        valid
    }

    /// Return `Vec` of validation errors.
    fn validation_errors(&self, cap: Option<&DeviceCapabilities>) -> Vec<Self::Error> {
        let mut v = Vec::new();
        self.validate(cap, |e| { v.push(e); });
        v
    }

    /// Ensure that values in `self` are valid at runtime. Returns a reference to `self`.
    fn expect_valid(&self, cap: Option<&DeviceCapabilities>, msg: &str) -> &Self {
        if !self.is_valid(cap) {
            expect_valid_inner(&self, msg, errors_to_str(self.validation_errors(cap)));
        }
        self
    }

    /// Ensure that values in `self` are valid at runtime. Returns a mutable reference to `self`.
    fn expect_valid_mut(&mut self, cap: Option<&DeviceCapabilities>, msg: &str) -> &mut Self {
        self.expect_valid(cap, msg);
        self
    }

    /// Almost the same as `expect_valid`, but doesn't perform validation on optimized builds.
    #[cfg(debug_assertions)]
    fn debug_expect_valid(&self, cap: Option<&DeviceCapabilities>, msg: &str) -> &Self {
        self.expect_valid(cap, msg)
    }

    /// Almost the same as `expect_valid_mut`, but doesn't perform validation on optimized builds.
    #[cfg(debug_assertions)]
    fn debug_expect_valid_mut(&mut self, cap: Option<&DeviceCapabilities>, msg: &str) -> &mut Self {
        self.expect_valid_mut(cap, msg)
    }

    #[cfg(not(debug_assertions))]
    fn debug_expect_valid(&self, cap: Option<&DeviceCapabilities>, msg: &str) -> &Self {
        self
    }

    #[cfg(not(debug_assertions))]
    fn debug_expect_valid_mut(&mut self, cap: Option<&DeviceCapabilities>, msg: &str) -> &mut Self {
        self
    }
}

fn expect_valid_inner(this: &Debug, msg: &str, err_str: String) -> ! {
    if msg.len() == 0 {
        panic!("Validation of the value {:#?} has failed. Reason: {}",
               this,
               err_str);
    } else {
        panic!("{}: Validation of the value {:#?} has failed. Reason: {}",
               msg,
               this,
               err_str);
    }
}

fn errors_to_str<T: Debug>(errors: Vec<T>) -> String {
    assert!(errors.len() > 0);
    let parts: Vec<String> = errors
        .iter()
        .map(|error| format!("{:?}", error))
        .collect();
    parts.join(", ")
}
