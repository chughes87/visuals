// Stub — blends the current frame with a previous-frame accumulation buffer
// to produce a motion-blur trail. Full implementation comes in Phase 6.

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

@group(0) @binding(0) var<uniform> u      : Uniforms;
@group(0) @binding(1) var          input  : texture_2d<f32>;
@group(0) @binding(2) var          output : texture_storage_2d<rgba32float, write>;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let px = vec2<i32>(i32(gid.x), i32(gid.y));
    if f32(gid.x) >= u.resolution.x || f32(gid.y) >= u.resolution.y { return; }
    // Pass-through until the accumulation buffer is wired up.
    let color = textureLoad(input, px, 0);
    textureStore(output, px, color);
}
