#version 310 es
precision mediump float;

layout(set = 0, binding = 0) uniform sampler2D unif_texture;
layout(location = 0) out vec4 output_color;

void main()
{
    output_color = vec4(texture(unif_texture, vec2(0.0)).x);
}
