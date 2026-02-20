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
struct EchoParams {
    layers : u32,
    offset : f32,
    decay  : f32,
    _pad   : f32,
}

@group(0) @binding(0) var<uniform>  u      : Uniforms;
@group(0) @binding(1) var<uniform>  ep     : EchoParams;
@group(0) @binding(2) var           input  : texture_2d<f32>;
@group(0) @binding(3) var           output : texture_storage_2d<rgba16float, write>;
@group(0) @binding(4) var           samp   : sampler;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let px     = vec2<f32>(f32(gid.x), f32(gid.y));
    if px.x >= u.resolution.x || px.y >= u.resolution.y { return; }

    var colour = vec4<f32>(0.0);
    var alpha  = 1.0;

    for (var l = 0u; l < ep.layers; l++) {
        let off    = f32(l) * ep.offset;
        let src_uv = (px + vec2(off, off)) / u.resolution;
        colour    += alpha * textureSampleLevel(input, samp, src_uv, 0.0);
        alpha     *= ep.decay;
    }

    textureStore(output, vec2<i32>(gid.xy), clamp(colour, vec4(0.0), vec4(1.0)));
}
