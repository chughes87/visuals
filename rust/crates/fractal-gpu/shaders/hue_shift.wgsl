struct HueParams {
    amount : f32,  // radians
    _pad   : vec3<f32>,
}

@group(0) @binding(0) var<uniform>  hp     : HueParams;
@group(0) @binding(1) var           input  : texture_2d<f32>;
@group(0) @binding(2) var           output : texture_storage_2d<rgba32float, write>;

// Rotate hue by cycling RGB channels with a rotation matrix in the
// luminance-preserving YIQ-like space.
fn hue_rotate(rgb: vec3<f32>, angle: f32) -> vec3<f32> {
    let c = cos(angle);
    let s = sin(angle);
    let w = vec3(0.299, 0.587, 0.114);
    let lum = dot(rgb, w);
    // Rodrigues rotation around the grey axis
    return vec3(
        lum + (rgb.r - lum) * c + (0.701 * rgb.r - 0.587 * rgb.g - 0.114 * rgb.b) * s,
        lum + (rgb.g - lum) * c + (-0.299 * rgb.r + 0.413 * rgb.g - 0.114 * rgb.b) * s,
        lum + (rgb.b - lum) * c + (-0.299 * rgb.r - 0.587 * rgb.g + 0.886 * rgb.b) * s,
    );
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let coord  = vec2<i32>(gid.xy);
    let px     = textureLoad(input, coord, 0);
    let shifted = hue_rotate(px.rgb, hp.amount);
    textureStore(output, coord, vec4<f32>(clamp(shifted, vec3(0.0), vec3(1.0)), px.a));
}
