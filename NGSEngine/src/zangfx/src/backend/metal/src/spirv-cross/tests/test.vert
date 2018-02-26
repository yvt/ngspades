#version 310 es

layout(set = 0, binding = 0) uniform sampler2D unif_texture;
layout(set = 0, binding = 1) uniform UBO1 {
    vec2 offset;
    int harmonic_coefficient[6];
} unif_buffer;
layout(set = 1, binding = 0) buffer SSBO1 {
    int arb_buf[];
} stor_buffer;

layout(location = 2) in vec4 hoge;
layout(location = 3) out vec4 piyo;

void main()
{
    gl_Position = hoge + texture(unif_texture, unif_buffer.offset);
    gl_Position.x += float(stor_buffer.arb_buf[0]);
    piyo = hoge;
}
