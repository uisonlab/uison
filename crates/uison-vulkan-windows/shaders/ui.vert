// One quad per instance: the shape's box (plus a pixel for antialiasing), cut by its clip.
// Coordinates are window pixels. Build: tools/vulkan-shaders.ps1 (glslc from the Vulkan SDK).
#version 450

layout(location = 0) in vec4 i_rect;   // box: x0 y0 x1 y1
layout(location = 1) in vec4 i_color;  // premultiplied; for textures, a multiplier
layout(location = 2) in vec4 i_extra;  // line: x1 y1 x2 y2; texture: u0 v0 u1 v1
layout(location = 3) in vec4 i_clip;   // x0 y0 x1 y1, whole pixels
layout(location = 4) in vec4 i_param;  // radius, stroke width, kind

layout(push_constant) uniform Push { vec2 size; } pc;

layout(location = 0) flat out vec4 v_rect;
layout(location = 1) flat out vec4 v_color;
layout(location = 2) flat out vec4 v_extra;
layout(location = 3) flat out vec4 v_param;

const vec2 CORNERS[6] = vec2[](vec2(0, 0), vec2(1, 0), vec2(0, 1), vec2(1, 0), vec2(1, 1), vec2(0, 1));

void main() {
    int kind = int(i_param.z + 0.5);
    float pad = 1.0;
    vec4 q = i_rect;
    if (kind == 2 || kind == 3) {
        q = vec4(min(i_extra.xy, i_extra.zw), max(i_extra.xy, i_extra.zw));
        pad += i_param.y * 0.5;
    } else if (kind == 1) {
        pad += i_param.y * 0.5;
    }
    q += vec4(-pad, -pad, pad, pad);
    q = vec4(max(q.xy, i_clip.xy), min(q.zw, i_clip.zw));
    q.zw = max(q.zw, q.xy);
    vec2 p = mix(q.xy, q.zw, CORNERS[gl_VertexIndex]);
    gl_Position = vec4(p / pc.size * 2.0 - 1.0, 0.0, 1.0);
    v_rect = i_rect;
    v_color = i_color;
    v_extra = i_extra;
    v_param = i_param;
}
