//
// Copyright 2018 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
//! Debug utiliites.

/// Trait for setting a debug label.
///
/// ZanGFX object types can implement this to accept debug labels. Builder types
/// must copy the labels to the objects built by them.
pub trait SetLabel {
    fn set_label(&mut self, label: &str);
}

/// Trait for setting a debug label on a ZanGFX object's trait object.
///
/// This calls [`SetLabel::set_label`] only if it's implemented (i.e., exposed
/// via `zangfx_impl_object!` or `interfaces!`).
///
/// [`SetLabel`]: SetLabel
/// [`SetLabel::set_label`]: SetLabel::set_label
///
/// # Examples
///
///     # use zangfx_base::*;
///     # fn test(device: &Device) {
///     let buffer = device.build_buffer()
///         .size(1024 * 1024)
///         .label("Pinkie mane vertex buffer")
///         .build()
///         .expect("Failed to create a buffer.");
///     # }
///
pub trait Label {
    fn label(&mut self, label: &str) -> &mut Self;
}
