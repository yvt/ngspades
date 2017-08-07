#version 310 es

layout(location = 0) in vec4 input_position;
layout(location = 1) in vec2 input_uv;

layout(location = 0) out mediump vec2 output_uv;

layout(set = 0, binding = 0) uniform SceneParams {
    mat4 view_proj_matrix;
} u_scene_params;

layout(set = 0, binding = 1) uniform ObjectParams {
    mat4 model_matrix;
} u_obj_params;

void main()
{
    gl_Position = u_scene_params.view_proj_matrix *
        u_obj_params.model_matrix *
        input_position;
    output_uv = input_uv;
}
