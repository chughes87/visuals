// Noise field — compute shader
//
// Approximates the Clojure NoiseGenerator which uses Quil/Processing Perlin
// noise at scale 0.01 animated with `time`.  Here we implement 4-octave FBM
// (fractional Brownian motion) with smooth value noise entirely in WGSL.
//
// Output: normalised noise value in the red channel [0, 1].

struct Uniforms {
    resolution: vec2<f32>,
    center:     vec2<f32>,
    zoom:       f32,
    time:       f32,
    max_iter:   u32,
    pad0:       u32,
    julia_c:    vec2<f32>,
    pad1:       vec2<f32>,
}

@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var output: texture_storage_2d<rgba32float, write>;

// ---------------------------------------------------------------------------
// Value noise helpers
// ---------------------------------------------------------------------------

// Hash a 2-D grid point to a pseudo-random scalar in [0, 1].
fn hash2(p: vec2<f32>) -> f32 {
    var q = fract(p * vec2<f32>(0.1031, 0.1030));
    q += dot(q, q.yx + 33.33);
    return fract((q.x + q.y) * q.x);
}

// Smooth (C² continuous) value noise.
fn vnoise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    // Quintic smoothstep for better continuity
    let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    return mix(
        mix(hash2(i + vec2<f32>(0.0, 0.0)), hash2(i + vec2<f32>(1.0, 0.0)), u.x),
        mix(hash2(i + vec2<f32>(0.0, 1.0)), hash2(i + vec2<f32>(1.0, 1.0)), u.x),
        u.y,
    );
}

// 4-octave FBM — matches Clojure NoiseGenerator's `octaves` parameter.
fn fbm(p: vec2<f32>) -> f32 {
    var value     = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;
    for (var oct = 0; oct < 4; oct++) {
        value     += amplitude * vnoise(p * frequency);
        frequency *= 2.0;
        amplitude *= 0.5;
    }
    return value;
}

// ---------------------------------------------------------------------------
// Shader entry point
// ---------------------------------------------------------------------------

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let px = vec2<f32>(f32(gid.x), f32(gid.y));
    if px.x >= u.resolution.x || px.y >= u.resolution.y { return; }

    // Map pixel → complex plane (same as other generators)
    let uv = (px - u.resolution * 0.5) / (u.zoom * u.resolution.y * 0.5);
    let p  = u.center + uv;

    // Scale to match Clojure's 0.01 pixel-scale at default zoom.
    // At zoom=1, uv spans ~[-1.33, 1.33] x [-1, 1]; * 3 gives noise coords
    // comparable to 0.01 * 800px = 8 units.
    let noise_scale = 3.0;
    // Animate with time in two directions (mimics 3-D Perlin's time axis)
    let animated = p * noise_scale + vec2<f32>(u.time * 0.10, u.time * 0.07);

    let n = fbm(animated);

    textureStore(output, vec2<i32>(gid.xy), vec4<f32>(n, 0.0, 0.0, 1.0));
}
