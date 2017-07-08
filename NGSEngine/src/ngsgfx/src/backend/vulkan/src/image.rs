//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use {core, RefEqArc, DeviceRef};

pub struct Image<T: DeviceRef> {
    data: RefEqArc<ImageData<T>>,
}

derive_using_field! {
    (T: DeviceRef); (PartialEq, Eq, Hash, Debug, Clone) for Image<T> => data
}

#[derive(Debug)]
struct ImageData<T: DeviceRef> {
    device: T,
}

impl<T: DeviceRef> core::Image for Image<T> {}

impl<T: DeviceRef> core::Marker for Image<T> {
    fn set_label(&self, label: Option<&str>) {
        // TODO: set_label
    }
}

pub struct ImageView<T: DeviceRef> {
    data: RefEqArc<ImageViewData<T>>,
}

derive_using_field! {
    (T: DeviceRef); (PartialEq, Eq, Hash, Debug, Clone) for ImageView<T> => data
}

#[derive(Debug)]
struct ImageViewData<T: DeviceRef> {
    device: T,
}

impl<T: DeviceRef> core::ImageView for ImageView<T> {}

impl<T: DeviceRef> core::Marker for ImageView<T> {
    fn set_label(&self, label: Option<&str>) {
        // TODO: set_label
    }
}
