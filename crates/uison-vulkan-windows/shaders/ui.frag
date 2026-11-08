// Coverage of the pixel by the shape, computed analytically: no MSAA, edges as smooth as Direct2D's.
// Kinds: 0 box fill, 1 box stroke, 2 line with flat caps, 3 line with round caps, 4 texture in a box.
#version 450

layout(location = 0) flat in vec4 v_rect;
layout(location = 1) flat in vec4 v_color;
layout(location = 2) flat in vec4 v_extra;
layout(location = 3) flat in vec4 v_param;

layout(set = 0, binding = 0) uniform sampler2D tex;

layout(location = 0) out vec4 o_color;

// How much of the pixel [p - 0.5, p + 0.5] lies in [lo, hi]: exact for axis-parallel edges.
float span(float p, float lo, float hi) {
    return clamp(min(p + 0.5, hi) - max(p - 0.5, lo), 0.0, 1.0);
}

float box(vec2 p, vec4 r, float radius) {
    if (r.z <= r.x || r.w <= r.y) return 0.0;
    if (radius <= 0.0) return span(p.x, r.x, r.z) * span(p.y, r.y, r.w);
    vec2 c = (r.xy + r.zw) * 0.5;
    vec2 b = (r.zw - r.xy) * 0.5;
    float rad = min(radius, min(b.x, b.y));
    vec2 q = abs(p - c) - b + rad;
    float d = length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - rad;
    return clamp(0.5 - d, 0.0, 1.0);
}

void main() {
    vec2 p = gl_FragCoord.xy;
    int kind = int(v_param.z + 0.5);
    float radius = v_param.x;
    float h = v_param.y * 0.5;
    vec4 color = v_color;
    float a;
    if (kind == 0) {
        a = box(p, v_rect, radius);
    } else if (kind == 1) {
        // The band between the outer and the inner edge; square corners stay square.
        float outer = box(p, v_rect + vec4(-h, -h, h, h), radius > 0.0 ? radius + h : 0.0);
        float inner = box(p, v_rect + vec4(h, h, -h, -h), max(radius - h, 0.0));
        a = outer - inner;
    } else if (kind == 2 || kind == 3) {
        vec2 d = v_extra.zw - v_extra.xy;
        float len = length(d);
        vec2 dir = len > 0.0 ? d / len : vec2(1.0, 0.0);
        vec2 rel = p - v_extra.xy;
        float u = dot(rel, dir);
        float v = dot(rel, vec2(-dir.y, dir.x));
        if (kind == 2) {
            a = span(u, 0.0, len) * span(v, -h, h);
        } else {
            a = span(length(vec2(u - clamp(u, 0.0, len), v)), -h, h);
        }
    } else {
        a = box(p, v_rect, radius);
        vec2 t = (p - v_rect.xy) / (v_rect.zw - v_rect.xy);
        color = texture(tex, mix(v_extra.xy, v_extra.zw, t)) * v_color;
    }
    o_color = color * a;
}
