#version 450 core
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
    vec3 f_normal = normalize(cross(dFdx(v_position), dFdy(v_position)));

    color = vec4(0,0,0,1);
    vec4 tempColor = vec4(0);

    //color = vec4(f_normal,0.3);
    tempColor = vec4(1);


    vec4 value = vec4(true_position.wwww);
    vec4 bound = vec4(sin(true_position.xyz*16+time)/8,true_position.w);
    vec4 upper = vec4(0.1);
    vec4 lower = vec4(0.1);

    bvec4 check = equal(lessThanEqual(value,bound+upper),lessThanEqual(bound-lower,value));
    if(check.x){color+=vec4(1,0,0,0);}
    if(check.y){color+=vec4(0,1,0,0);}
    if(check.z){color+=vec4(0,0,1,0);}
    if(check.w){color+=vec4(0,0,0,0);}


    //color = vec4(1,1,1,0.3);
    //color = vec4(abs(true_normal));
    //color = vec4(v_normal,1);
}