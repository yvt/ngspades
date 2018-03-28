#version 440
#extension GL_GOOGLE_include_directive : enable
//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
#include "composite.h"

precision mediump float;

layout(location = 0) in highp vec3 input_uv;
layout(location = 1) in flat lowp uint input_straight_alpha;
layout(location = 2) in flat vec4 input_color;
layout(location = 0) out vec4 output_color;

layout(set = 1, binding = 0) uniform texture2D u_image;
layout(set = 1, binding = 1) uniform mediump sampler u_imageSampler;
layout(set = 1, binding = 2) uniform texture2D u_mask;
layout(set = 1, binding = 3) uniform mediump sampler u_maskSampler;

void main() {
    vec2 uv = input_uv.xy / input_uv.z;
    vec4 color = texture(sampler2D(u_image, u_imageSampler), uv);

    // Convert to pre-multipled alpha
    if (input_straight_alpha != 0u) {
        color.xyz *= color.w;
    }

    // Apply mask
    color *= texture(sampler2D(u_mask, u_maskSampler), uv).w;

    // Apply color modulation
    color *= input_color;

    output_color = color;
}
