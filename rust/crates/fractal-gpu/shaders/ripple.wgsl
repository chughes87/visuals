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
struct RippleParams {
    frequency : f32,
    amplitude : f32,
    speed     : f32,
    _pad      : f32,
}

@group(0) @binding(0) var<uniform>  u      : Uniforms;
@group(0) @binding(1) var<uniform>  rp     : RippleParams;
@group(0) @binding(2) var           input  : texture_2d<f32>;
@group(0) @binding(3) var           output : texture_storage_2d<rgba16float, write>;
@group(0) @binding(4) var           samp   : sampler;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let px  = vec2<f32>(f32(gid.x), f32(gid.y));
    if px.x >= u.resolution.x || px.y >= u.resolution.y { return; }

    let t   = u.time * rp.speed;
    let dx  = rp.amplitude * sin(px.y * rp.frequency + t);
    let dy  = rp.amplitude * sin(px.x * rp.frequency + t * 1.5);

    let src_uv = (px + vec2(dx, dy)) / u.resolution;
    let colour = textureSampleLevel(input, samp, src_uv, 0.0);

    textureStore(output, vec2<i32>(gid.xy), colour);
}
