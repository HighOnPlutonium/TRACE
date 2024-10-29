#version 460
//#extension GL_NV_fragment_shader_barycentric : enable
//#extension GL_ARB_gpu_shader5 : enable
//#extension GL_NV_gpu_shader5 : enable


layout(location=0, index=0) out vec4 output1;
layout(location=0, index=1) out vec4 output2;

in vec4 true_position;
//pervertexNV in vec4 true_normal_PV[];
in vec4 true_normal;

in vec3 v_position;
in vec3 v_normal;
in float dist;


//uniform u32vec2 resolution;
uniform ivec2 resolution;
uniform float time;



void main() {
    vec3 f_normal = normalize(cross(dFdx(v_position), dFdy(v_position)));


    vec4 color;
    vec4 auxiliary = vec4(0);


    color = vec4(1,1,1,0.7);
    auxiliary = vec4(f_normal,1);


    output1 = color;
    output2 = auxiliary;
}