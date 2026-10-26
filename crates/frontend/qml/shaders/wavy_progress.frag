#version 440
// Material 3 Expressive's wavy linear progress: the done part a sine wave,
// a gap, the rest a flat track ending in a stop dot. With `progress` at 1 and no
// track it is the squiggle that runs on from a section's title.

layout(location = 0) in vec2 qt_TexCoord0;
layout(location = 0) out vec4 fragColor;

layout(std140, binding = 0) uniform buf {
    mat4 qt_Matrix;
    float qt_Opacity;
    vec2 itemSize;
    vec4 color;
    vec4 trackColor;
    // 0..1 of the length
    float progress;
    float thickness;
    float amplitude;
    float wavelength;
    float phase;
    // pixels between the wave's end cap and the track's
    float gap;
};

const float TAU = 6.28318530718;

// Coverage of a round-capped stroke along the sine from x0 to x1.
float wave(vec2 p, float x0, float x1, float half_)
{
    if (x1 < x0)
        return 0.0;
    float x = clamp(p.x, x0, x1);
    float angle = TAU * x / wavelength + phase;
    float y = amplitude * sin(angle);
    float slope = amplitude * TAU / wavelength * cos(angle);
    vec2 offset = vec2(p.x - x, (p.y - y) / sqrt(1.0 + slope * slope));
    return clamp(half_ + 0.5 - length(offset), 0.0, 1.0);
}

// Coverage of a round-capped straight stroke from x0 to x1.
float flat_(vec2 p, float x0, float x1, float half_)
{
    if (x1 < x0)
        return 0.0;
    vec2 offset = vec2(p.x - clamp(p.x, x0, x1), p.y);
    return clamp(half_ + 0.5 - length(offset), 0.0, 1.0);
}

void main()
{
    float half_ = 0.5 * thickness;
    vec2 p = vec2(qt_TexCoord0.x * itemSize.x, (qt_TexCoord0.y - 0.5) * itemSize.y);
    float start = half_;
    float end = itemSize.x - half_;
    float reach = mix(start, end, clamp(progress, 0.0, 1.0));
    // Nothing done yet is no wave at all, not a dot.
    float done = progress > 0.001 ? wave(p, start, reach, half_) : 0.0;
    float track = 0.0;
    if (trackColor.a > 0.0) {
        float trackStart = progress > 0.001 ? reach + gap + thickness : start;
        track = flat_(p, trackStart, end, half_);
    }
    vec4 result = trackColor * track * (1.0 - done) + color * done;
    // The stop dot at the end, in the active colour, while the track shows.
    if (trackColor.a > 0.0 && progress < 0.98) {
        float dot_ = clamp(0.25 * thickness + 0.5 - length(p - vec2(end, 0.0)), 0.0, 1.0);
        result = mix(result, color, dot_);
    }
    fragColor = result * qt_Opacity;
}
