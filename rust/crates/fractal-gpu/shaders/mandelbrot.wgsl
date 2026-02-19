// Mandelbrot set — compute shader
//
// For each pixel, iterate z = z² + c where c is the point on the complex plane
// and z starts at 0.  Store a smooth normalised iteration count in the red
// channel of the output texture.  The colour_map effect shader converts this
// [0, 1] value to an RGB colour.

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

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let px = vec2<f32>(f32(gid.x), f32(gid.y));
    if px.x >= u.resolution.x || px.y >= u.resolution.y { return; }

    // Map pixel → complex plane
    let uv = (px - u.resolution * 0.5) / (u.zoom * u.resolution.y * 0.5);
    let c  = u.center + uv;

    var z  = vec2<f32>(0.0, 0.0);
    var i  = 0u;
    while i < u.max_iter {
        if dot(z, z) > 4.0 { break; }
        z = vec2<f32>(z.x * z.x - z.y * z.y + c.x, 2.0 * z.x * z.y + c.y);
        i++;
    }

    // Interior points → 0.  Escaped points → smooth normalised count.
    var t = 0.0;
    if i < u.max_iter {
        // log-log escape smoothing (Inigo Quilez)
        let log_zn = log2(max(dot(z, z), 1e-10)) * 0.5;
        let nu     = log2(max(log_zn, 1e-10));
        t = clamp((f32(i) + 1.0 - nu) / f32(u.max_iter), 0.0, 1.0);
    }

    textureStore(output, vec2<i32>(gid.xy), vec4<f32>(t, 0.0, 0.0, 1.0));
}
