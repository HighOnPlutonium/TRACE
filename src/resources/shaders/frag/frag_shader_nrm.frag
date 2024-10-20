#version 450 core
//#extension GL_NV_fragment_shader_barycentric : enable
//#extension GL_ARB_gpu_shader5 : enable
#extension GL_NV_gpu_shader5 : enable

out vec4 color;

in vec4 true_position;
//pervertexNV in vec4 true_normal_PV[];
in vec4 true_normal;

in vec3 v_position;
in vec3 v_normal;
in float dist;


uniform u32vec2 resolution;
uniform float time;





void main() {
    vec3 f_normal = normalize(cross(dFdx(v_position), dFdy(v_position)));

    color = vec4(f_normal,0.3);

    //color = vec4(1,1,1,0.3);
    //color = vec4(abs(true_normal));
    //color = vec4(v_normal,1);
}