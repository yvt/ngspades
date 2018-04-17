#version 310 es

layout(location = 0) in vec3 input_position;
layout(location = 1) in vec3 input_color;

layout(location = 0) out mediump vec4 output_color;

void main()
{
    gl_Position = vec4(input_position, 1.0);
    output_color = vec4(input_color, 1.0);

    float angle = float(gl_InstanceIndex) / 65536.0 * 3.141592654 * 2.0;
    mat2 m = mat2(cos(angle), -sin(angle), sin(angle), cos(angle));
    gl_Position.xy = m * gl_Position.xy;
}
