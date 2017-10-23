#version 310 es
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

layout(set = 1, binding = 0) uniform sampler2D u_image;
layout(set = 1, binding = 1) uniform sampler2D u_mask;

void main() {
    vec2 uv = input_uv.xy / input_uv.z;
    vec4 color = texture(u_image, uv);

    // Convert to pre-multipled alpha
    if (input_straight_alpha != 0u) {
        color.xyz *= color.w;
    }

    // Apply mask
    color *= texture(u_mask, uv).w;

    // Apply color modulation
    color *= input_color;

    output_color = color;
}
