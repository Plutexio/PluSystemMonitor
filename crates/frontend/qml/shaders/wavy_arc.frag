#version 440
// Material 3 Expressive's wavy circular progress, opened at the bottom like a
// gauge: the done part rides a wave round the ring, a gap, then a flat track
// to a stop dot at the far end.

layout(location = 0) in vec2 qt_TexCoord0;
layout(location = 0) out vec4 fragColor;

layout(std140, binding = 0) uniform buf {
    mat4 qt_Matrix;
    float qt_Opacity;
    vec2 itemSize;
    vec4 color;
    vec4 trackColor;
    // 0..1 of the sweep
    float progress;
    float radius;
    float thickness;
    float amplitude;
    // whole waves round a full circle
    float waves;
    // where the ring starts and how far it goes, in radians, clockwise from
    // the right
    float startAngle;
    float sweep;
    // pixels between the wave's end cap and the track's
    float gap;
};

const float TAU = 6.28318530718;

vec2 at(float angle, float r)
{
    return vec2(cos(angle), sin(angle)) * r;
}

void main()
{
    float half_ = 0.5 * thickness;
    vec2 p = qt_TexCoord0 * itemSize - 0.5 * itemSize;
    float len = length(p);
    // How far round from the start this point is.
    float s = mod(atan(p.y, p.x) - startAngle + 2.0 * TAU, TAU);
    float reach = sweep * clamp(progress, 0.0, 1.0);
    // The wave fades in over the first part, so a small progress is a short
    // stroke rather than a stub of a wave.
    float a = amplitude * smoothstep(0.0, 0.6, reach);

    float done = 0.0;
    if (progress > 0.001) {
        float k = waves;
        if (s <= reach) {
            float angle = startAngle + s;
            float r = radius + a * sin(k * angle);
            float slope = a * k * cos(k * angle) / max(len, 1.0);
            done = clamp(half_ + 0.5 - abs(len - r) / sqrt(1.0 + slope * slope), 0.0, 1.0);
        }
        vec2 from = at(startAngle, radius + a * sin(k * startAngle));
        vec2 to = at(startAngle + reach, radius + a * sin(k * (startAngle + reach)));
        done = max(done, clamp(half_ + 0.5 - length(p - from), 0.0, 1.0));
        done = max(done, clamp(half_ + 0.5 - length(p - to), 0.0, 1.0));
    }

    float track = 0.0;
    float trackStart = progress > 0.001 ? reach + (gap + thickness) / radius : 0.0;
    if (trackStart < sweep) {
        if (s >= trackStart && s <= sweep)
            track = clamp(half_ + 0.5 - abs(len - radius), 0.0, 1.0);
        track = max(track, clamp(half_ + 0.5 - length(p - at(startAngle + trackStart, radius)), 0.0, 1.0));
        track = max(track, clamp(half_ + 0.5 - length(p - at(startAngle + sweep, radius)), 0.0, 1.0));
    }

    vec4 result = trackColor * track * (1.0 - done) + color * done;
    if (progress < 0.98) {
        float dot_ = clamp(0.25 * thickness + 0.5 - length(p - at(startAngle + sweep, radius)), 0.0, 1.0);
        result = mix(result, color, dot_);
    }
    fragColor = result * qt_Opacity;
}
