#version 310 es
precision mediump float;

layout(location = 0) in vec2 input_uv;
layout(location = 0) out vec4 output_color;

layout(set = 0, binding = 2) uniform sampler2D u_texture;

void main()
{
    output_color = texture(u_texture, input_uv);
}
