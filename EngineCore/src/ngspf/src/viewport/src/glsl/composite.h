//
// Copyright 2017 yvt, all rights reserved.
//
// This source code is a part of Nightingales.
//

struct Sprite {
    highp mat4 matrix;
    highp mat4 uv_matrix;
    highp vec4 color;
    highp uint flags;
    highp uint _pad1;
    highp uint _pad2;
    highp uint _pad3;
};

/// Sprite::flags
#define SF_STRAIGHT_ALPHA   0x00000001u

layout(std430, set = 0, binding = 0) readonly buffer SpriteParams {
    Sprite sprites[];
} u_sprite_params;
