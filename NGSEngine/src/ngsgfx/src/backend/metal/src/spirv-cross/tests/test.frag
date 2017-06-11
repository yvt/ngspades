#version 310 es
precision mediump float;

layout(set = 0, binding = 0) uniform sampler2D unif_texture;
layout(set = 0, binding = 1) uniform UBO1 {
    vec2 offset;
} unif_buffer;

layout(location = 0) in vec2 input_uv_coordinate;
layout(location = 0) out vec4 output_color;

void main()
{
    output_color = texture(unif_texture, input_uv_coordinate + unif_buffer.offset);
}
