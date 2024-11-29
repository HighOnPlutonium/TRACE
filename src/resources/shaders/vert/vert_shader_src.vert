#version 450 core


in vec4 position;
in vec4 normal;
in vec4 tex;
out vec4 tex_c;


out vec4 true_position;
out vec4 true_normal_PV;

out vec3 v_position;
out vec3 v_normal;
out vec4 true_normal;
out float dist;

uniform mat4 scale;
uniform mat4 speen;
uniform vec4 proj_point;

uniform mat4 xz_yw_rot;

uniform float time;






void main() {

    mat4 t4 = xz_yw_rot;
    //t4 = mat4(1);
    mat4 t3 = speen;
    //t3 = mat4(1);

    vec4 t_position = t4 * position;
    vec4   t_normal = t4 * normal;
               dist = distance(t_position, proj_point)-0.8;
    vec3 p_position = vec3(t_position * dist)*1.2;
    vec3   p_normal = vec3(t_normal * dist);

    true_position = position;
      true_normal = t_normal;

       v_normal = transpose(inverse(mat3(t3))) * p_normal;
    gl_Position = scale * t3 * vec4(p_position, 1);
     v_position = gl_Position.xyz / gl_Position.w;

    tex_c = tex;
}