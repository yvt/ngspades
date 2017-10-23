#version 310 es
#extension GL_GOOGLE_include_directive : enable
//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//
#include "composite.h"

layout(location = 0) in uvec2 input_position;

layout(location = 0) out highp vec3 output_uv;
layout(location = 1) out lowp uint output_straight_alpha;
layout(location = 2) out mediump vec4 output_color;

void main() {
    Sprite s = u_sprite_params.sprites[gl_InstanceIndex];

    vec4 pos = vec4(input_position, 0.0, 1.0);

    gl_Position = s.matrix * pos;

    output_uv = (s.uv_matrix * pos).xyw;
    output_straight_alpha = uint(s.flags);
    output_color = s.color;
}
