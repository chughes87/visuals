// Motion blur — blends the current frame with a history accumulation buffer.
// The accumulation texture is wired up in Phase 6; for now this is a
// pass-through that compiles and dispatches correctly.

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
struct MotionBlurParams {
    opacity : f32,
    _pad0   : f32,
    _pad1   : f32,
    _pad2   : f32,
}

@group(0) @binding(0) var<uniform>  u      : Uniforms;
@group(0) @binding(1) var<uniform>  mp     : MotionBlurParams;
@group(0) @binding(2) var           input  : texture_2d<f32>;
@group(0) @binding(3) var           output : texture_storage_2d<rgba16float, write>;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let px = vec2<i32>(i32(gid.x), i32(gid.y));
    if f32(gid.x) >= u.resolution.x || f32(gid.y) >= u.resolution.y { return; }
    // Pass-through — accumulation buffer blending wired in Phase 6.
    let color = textureLoad(input, px, 0);
    textureStore(output, px, color);
}
