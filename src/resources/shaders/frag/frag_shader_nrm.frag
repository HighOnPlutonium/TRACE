#version 460
//#extension GL_NV_fragment_shader_barycentric : enable
//#extension GL_ARB_gpu_shader5 : enable
//#extension GL_NV_gpu_shader5 : enable


out vec4 color;

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
    float t = radians(time * 120);

    vec3 f_normal = normalize(cross(dFdx(v_position), dFdy(v_position)));

    color = vec4(pow(smoothstep(-1,1,true_position.www),vec3(2)),0.5);

}