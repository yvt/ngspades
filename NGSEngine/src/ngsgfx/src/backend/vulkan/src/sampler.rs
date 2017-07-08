//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;

use {RefEqArc, DeviceRef};

pub struct Sampler<T: DeviceRef> {
    data: RefEqArc<SamplerData<T>>,
}

derive_using_field! {
    (T: DeviceRef); (PartialEq, Eq, Hash, Debug, Clone) for Sampler<T> => data
}

#[derive(Debug)]
struct SamplerData<T: DeviceRef> {
    device: T,
}

impl<T: DeviceRef> core::Sampler for Sampler<T> {}

impl<T: DeviceRef> core::Marker for Sampler<T> {
    fn set_label(&self, label: Option<&str>) {
        // TODO: set_label
    }
}
