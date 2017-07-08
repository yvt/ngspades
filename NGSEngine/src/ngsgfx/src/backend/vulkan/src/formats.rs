//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
use core::{ImageFormat, Signedness, Normalizedness, VertexFormat, VectorWidth, ScalarFormat};
use self::Signedness::{Signed, Unsigned};
use self::Normalizedness::{Normalized, Unnormalized};
use self::VectorWidth::{Scalar, Vector2, Vector3, Vector4};
use self::ScalarFormat::{I8, I16, I32, F32};
use ash::vk;

pub fn translate_image_format(format: ImageFormat) -> vk::Format {
    unimplemented!()
}

pub fn translate_vertex_format(format: VertexFormat) -> vk::Format {
    unimplemented!()
}
