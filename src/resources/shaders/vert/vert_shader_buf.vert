#version 150 core

in vec4 position;
in vec4 normal;

out vec3 v_position;


void main() {
    gl_Position = vec4(position.xyz,1);
    v_position = gl_Position.xyz / gl_Position.w;
}