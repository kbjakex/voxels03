#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(push_constant) uniform PerFrame {
    mat4 mvp;
} per_frame;

layout(set = 0, binding = 0) readonly buffer Faces {
	uint arr[];
} faces;

vec3 offsetCcw(uint i, uint face_mask) {
    float x = (0xCA0CA0C >> i >> ((face_mask - 1) << 2)) & 1;
    float y = (0xCA0CA0C >> i >> ((face_mask    ) << 2)) & 1;
    float z = (0xCA0CA0C >> i >> ((face_mask + 1) << 2)) & 1;
    return vec3(x, y, z);
}

vec3 offsetCw(uint i, uint face_mask) {
    float x = (0xAC0AC0A >> i >> ((face_mask - 1) << 2)) & 1;
    float y = (0xAC0AC0A >> i >> ((face_mask    ) << 2)) & 1;
    float z = (0xAC0AC0A >> i >> ((face_mask + 1) << 2)) & 1;
    return vec3(x, y, z);
}

layout(location = 0) out vec3 aColor;

void main() {
    uint face = faces.arr[gl_VertexIndex >> 2];

    // MSB [XXXX XYYY][YYZZ ZZZF][NN?? ??II][IIII IIII] LSB
    vec3 pos = vec3(face >> 27, (face >> 22) & 0x1F, (face >> 17) & 0x1F);
    bool flip = (face & 0x10000) != 0; // 1 << 16 = 0x10000
    uint normal_bits = (face >> 14) & 3;
    uint tex_id = face & 0x3FF;

    uint v_idx = gl_VertexIndex & 3;
    if (flip) {
        pos += 1.0 - offsetCcw(v_idx, normal_bits);
    } else {
        pos += offsetCw(v_idx, normal_bits);
    }

    gl_Position = per_frame.mvp * vec4(pos, 1.0);
    aColor = pos / 32.0;
}
