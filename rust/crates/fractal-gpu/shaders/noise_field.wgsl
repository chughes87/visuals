// Simple value noise — a GPU-native replacement for Quil's Perlin noise.
// Uses a hash-based approach: fast, no texture lookups needed.

struct Uniforms {
    resolution : vec2<f32>,
    center     : vec2<f32>,
    zoom       : f32,
    time       : f32,
    max_iter   : u32,
    _pad       : u32,
    julia_c    : vec2<f32>,
    _pad2      : vec2<f32>,
}

@group(0) @binding(0) var<uniform>        u      : Uniforms;
@group(0) @binding(1) var                 output : texture_storage_2d<rgba32float, write>;

fn hash2(p: vec2<f32>) -> f32 {
    let q = vec2<f32>(dot(p, vec2<f32>(127.1, 311.7)),
                      dot(p, vec2<f32>(269.5, 183.3)));
    return fract(sin(q.x + q.y) * 43758.5453);
}

fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f); // smoothstep
    return mix(mix(hash2(i + vec2(0.0, 0.0)), hash2(i + vec2(1.0, 0.0)), u.x),
               mix(hash2(i + vec2(0.0, 1.0)), hash2(i + vec2(1.0, 1.0)), u.x),
               u.y);
}

// 4-octave fBm
fn fbm(p: vec2<f32>) -> f32 {
    var v = 0.0;
    var a = 0.5;
    var q = p;
    for (var o = 0; o < 4; o++) {
        v += a * noise(q);
        q  = q * 2.0 + vec2(1.7, 9.2);
        a *= 0.5;
    }
    return v;
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let px = vec2<f32>(f32(gid.x), f32(gid.y));
    if px.x >= u.resolution.x || px.y >= u.resolution.y { return; }

    let uv = px / u.resolution;
    let t  = fbm(uv * 4.0 + u.time * 0.2);

    textureStore(output, vec2<i32>(gid.xy), vec4<f32>(t, 0.0, 0.0, 1.0));
}
