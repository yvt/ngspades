#version 310 es

layout(location = 2) in vec4 hoge;
layout(location = 3) out vec4 piyo;

void main()
{
    gl_Position = hoge;
    piyo = hoge;
}
