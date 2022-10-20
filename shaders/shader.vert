#version 450

layout(set = 0, binding = 0) uniform GlobalUniformBufferObject {
    mat4 model;
    mat4 view;
    mat4 proj;
} global;

layout(set = 1, binding = 0) uniform ModelUniformBufferObject {
    mat4 model;
    mat4 view;
    mat4 proj;
} model;

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inColor;
layout(location = 2) in vec2 inTexCoord;

layout(location = 0) out vec3 fragColor;
layout(location = 1) out vec2 fragTexCoord;

void main() {
    gl_Position = global.proj * global.view * model.model * vec4(inPosition, 1.0);
    fragColor = inColor;
    fragTexCoord = inTexCoord;
}
