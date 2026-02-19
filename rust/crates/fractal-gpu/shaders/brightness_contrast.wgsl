struct BCParams {
    brightness : f32,
    contrast   : f32,
    _pad       : vec2<f32>,
}

@group(0) @binding(0) var<uniform>  bp     : BCParams;
@group(0) @binding(1) var           input  : texture_2d<f32>;
@group(0) @binding(2) var           output : texture_storage_2d<rgba32float, write>;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let coord = vec2<i32>(gid.xy);
    let px    = textureLoad(input, coord, 0);
    let rgb   = clamp((px.rgb + bp.brightness) * bp.contrast, vec3(0.0), vec3(1.0));
    textureStore(output, coord, vec4<f32>(rgb, px.a));
}
