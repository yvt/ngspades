//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core;

use {RefEqArc, DeviceRef};

pub struct GraphicsPipeline<T: DeviceRef> {
    data: RefEqArc<GraphicsPipelineData<T>>,
}

derive_using_field! {
    (T: DeviceRef); (PartialEq, Eq, Hash, Debug, Clone) for GraphicsPipeline<T> => data
}

#[derive(Debug)]
struct GraphicsPipelineData<T: DeviceRef> {
    device: T,
}

impl<T: DeviceRef> core::GraphicsPipeline for GraphicsPipeline<T> {}


pub struct StencilState<T: DeviceRef> {
    data: RefEqArc<StencilStateData<T>>,
}

derive_using_field! {
    (T: DeviceRef); (PartialEq, Eq, Hash, Debug, Clone) for StencilState<T> => data
}

#[derive(Debug)]
struct StencilStateData<T: DeviceRef> {
    device: T,
}

impl<T: DeviceRef> core::StencilState for StencilState<T> {}


pub struct ComputePipeline<T: DeviceRef> {
    data: RefEqArc<ComputePipelineData<T>>,
}

derive_using_field! {
    (T: DeviceRef); (PartialEq, Eq, Hash, Debug, Clone) for ComputePipeline<T> => data
}

#[derive(Debug)]
struct ComputePipelineData<T: DeviceRef> {
    device: T,
}

impl<T: DeviceRef> core::ComputePipeline for ComputePipeline<T> {}
