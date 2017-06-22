#version 310 es
precision mediump float;

layout(location = 0) in vec4 input_color;
layout(location = 0) out vec4 output_color;

void main()
{
    output_color = input_color;
}
