#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec3 fragColor;
layout(location = 0) out vec4 outColor;

void main() {
    vec2 pc = gl_PointCoord * 2. - 1.;

    vec3 color = fragColor;

    if (length(fragColor) < 0.1) discard;

    color -= 0.1;
    color *= (1. + 0.1);
    //if (length(pc) > 1. || length(pc) < 0.5) discard;

    outColor = vec4(color, 1.);
}

