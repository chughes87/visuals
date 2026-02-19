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

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let px = vec2<f32>(f32(gid.x), f32(gid.y));
    if px.x >= u.resolution.x || px.y >= u.resolution.y { return; }

    // For Julia: z starts at pixel position, c is fixed (julia_c uniform)
    let uv = (px - u.resolution * 0.5) / (u.zoom * u.resolution.y * 0.5);
    var z  = u.center + uv;
    let c  = u.julia_c;

    var i = 0u;
    loop {
        if i >= u.max_iter || dot(z, z) > 4.0 { break; }
        z = vec2<f32>(z.x * z.x - z.y * z.y + c.x,
                      2.0 * z.x * z.y         + c.y);
        i++;
    }

    var smooth_i: f32;
    if i < u.max_iter {
        smooth_i = f32(i) + 1.0 - log2(log2(dot(z, z)));
    } else {
        smooth_i = f32(u.max_iter);
    }
    let t = smooth_i / f32(u.max_iter);

    textureStore(output, vec2<i32>(gid.xy), vec4<f32>(t, 0.0, 0.0, 1.0));
}
