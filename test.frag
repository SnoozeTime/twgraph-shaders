#version 450

layout(location = 0) out vec4 f_color;
layout(location = 0) in vec4 frag_color;
layout(location = 1) in vec2 frag_tex_coords;
layout(location = 2) in vec3 frag_position;
layout(location = 3) in vec3 frag_normal;

layout(set = 1, binding = 0) uniform sampler2D texSampler;
layout(set = 1, binding = 1) uniform Data {
        vec3 color;
        vec3 position;
} light;


void main() {
        // ambient
        float ambientStrength = 0.1;
        vec3 ambient = ambientStrength * light.color 
                * texture(texSampler, frag_tex_coords).rgb;

        // diffuse 
        vec3 norm = normalize(frag_normal);
        vec3 lightDir = normalize(light.position - frag_position);
        float diff = max(dot(norm, lightDir), 0.0);
        vec3 diffuse = diff * light.color 
                * texture(texSampler, frag_tex_coords).rgb;

        vec3 result = (ambient + diffuse);
        //f_color = texture(texSampler, frag_tex_coords);
        f_color = vec4(result, 1.0);
}
