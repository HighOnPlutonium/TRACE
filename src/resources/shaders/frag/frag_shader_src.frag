#version 150 core

out vec4 color;

in vec3 v_position;
in vec3 v_normal;
in float dist;
in vec4 true_normal;

uniform vec3 u_light;




const vec3 ambient_color = vec3(0.15, 0.15, 0.15);
const vec3 diffuse_color = vec3(0.55, 0.55, 0.55);
const vec3 specular_color = vec3(0.95, 0.95, 0.95);

void main() {

    vec3 f_normal = normalize(cross(dFdx(v_position), dFdy(v_position)));
    //f_normal = v_normal/abs(dist);

    vec3 camera_dir = normalize(-v_position);
    vec3 half_direction = normalize(normalize(u_light) + camera_dir);

    float diffuse = max(dot(normalize(f_normal), normalize(u_light)), 0.0);
    float specular = pow(max(dot(half_direction, normalize(f_normal)), 0.0), 16.0);

    color = vec4((ambient_color + diffuse * diffuse_color + specular * specular_color)/1, 0.7);

    //color = (color + vec4(vec3(pow(dist,2)*3),0.5)*0.1)*0.7;
    //color += vec4(f_normal,-0.125);
    float a = smoothstep(0.4,1,dist);
    float b =smoothstep(0.4,0.9,1/(a+1));
    color = vec4(vec3(smoothstep(0.1,1,b)),b);
}