#version 310 es
precision mediump float;

layout(constant_id = 0) const int SOME_CONSTANT = 114514;

layout(set = 0, binding = 0) uniform sampler2D unif_texture;
layout(set = 0, binding = 1) uniform UBO1 {
    vec2 offset;
    int harmonic_coefficient[6];
} unif_buffer;
layout(set = 1, binding = 0) buffer SSBO1 {
    int arb_buf[];
} stor_buffer;

layout(location = 0) in vec2 input_uv_coordinate;
layout(location = 0) out vec4 output_color;

void main()
{
    output_color = texture(unif_texture, input_uv_coordinate + unif_buffer.offset)
        + float(stor_buffer.arb_buf[SOME_CONSTANT]);
}
