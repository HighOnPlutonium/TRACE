#version 450 core

out vec4 color;

uniform sampler2D tex;
uniform sampler2D kernel;
uniform float time;

uniform vec2 resolution;


vec4 convolve(sampler2D kernel, float scale) {
    color = vec4(0,0,0,0);
    ivec2 size = textureSize(kernel,0);
    ivec2 coords = ivec2(gl_FragCoord.xy
                      + (1-size)*scale/2);

    for (int y=0;y<size.y;y++) {
    for (int x=0;x<size.x;x++) {

         color += texelFetch(kernel, ivec2(x,y), 0).x
                * texelFetch(tex, coords + ivec2(roundEven(x*scale),
                                                 roundEven(y*scale)), 0);
    }} return color;
}


void main() {
    //vec4 value = convolve(kernel,1.0);
    //value = smoothstep(0,0.1,value);
    //value = vec4(vec3(value.x+value.y+value.z),value.w);
    //vec4 color_adj = value;
    vec4 color_buf = texelFetch(tex,ivec2(gl_FragCoord),0);
    //vec4 color_dab = vec4(max(color_adj-color_buf,0).xyz,1);
    //vec4 color_dba = vec4(max(color_buf-color_adj,0).xyz,1);

    //color = vec4(0,0,0,0);
    //float time = time * 1.0;
    //if (mod(time+1,     4) > 3.5) {color += color_dba;}
    //if (mod(time+1.5, 4) > 3.5) {color += color_dab;}
    //if (mod(time+2, 4) > 3.5) {color += color_adj;}
    //if (mod(time + 2.5, 4) > 1.5) {color += color_buf;}

    color = color_buf;
}