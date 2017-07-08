//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;

use {RefEqArc, DeviceRef};

pub struct ShaderModule<T: DeviceRef> {
    data: RefEqArc<ShaderModuleData<T>>,
}

derive_using_field! {
    (T: DeviceRef); (PartialEq, Eq, Hash, Debug, Clone) for ShaderModule<T> => data
}

#[derive(Debug)]
struct ShaderModuleData<T: DeviceRef> {
    device: T,
}

impl<T: DeviceRef> core::ShaderModule for ShaderModule<T> {}

impl<T: DeviceRef> core::Marker for ShaderModule<T> {
    fn set_label(&self, label: Option<&str>) {
        // TODO: set_label
    }
}
