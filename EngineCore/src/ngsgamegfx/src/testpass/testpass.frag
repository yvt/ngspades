#version 310 es
precision mediump float;

layout(location = 0) in highp vec2 input_position;
layout(location = 0) out vec4 output_color;

const float DIVERGE_THRESHOLD = 65536.0;

vec2 complexMul(vec2 a, vec2 b) {
    return vec2(
        a.x * b.x - a.y * b.y,
        a.x * b.y + a.y * b.x
    );
}

void main()
{
    // Mandelbrot set
    vec2 z = input_position;

    float i = 0.0;
    while (i < 128.0) {
        vec2 z_next = complexMul(z, z) + input_position;

        if (dot(z_next, z_next) > DIVERGE_THRESHOLD) {
            float z_mag = log2(dot(z, z));
            float z_next_mag = log2(dot(z_next, z_next));
            i += (log2(DIVERGE_THRESHOLD) - z_mag) / (z_next_mag - z_mag);
            break;
        } else {
            i += 1.0;
        }

        z = z_next;
    }

    output_color.xyz = sin(vec3(i / 128.0) * vec3(3.0, 6.0, 32.0));
    output_color.xyz = mix(output_color.xyz, vec3(1.0), pow(i / 128.0, 32.0));
    output_color.xyz = pow(output_color.xyz, vec3(4.0));
    output_color.w = 1.0;
}
