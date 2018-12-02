#version 310 es

layout(location = 0) in uvec2 input_position;
layout(location = 0) out highp vec2 output_position;

void main()
{
	vec2 position = vec2(input_position);
    gl_Position = vec4(position * 2.0 - 1.0, 0.5, 1.0);
    output_position = (position - 0.5) * 2.5 + vec2(-0.5, 0.0);
}
