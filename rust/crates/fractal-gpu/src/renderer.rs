/// Full-screen quad renderer — samples the final effect texture and
/// presents it to the wgpu Surface.
///
/// The vertex shader generates a clip-space quad from vertex indices
/// (no vertex buffer needed). The fragment shader simply samples the
/// texture produced by the effect chain.
pub const FULLSCREEN_WGSL: &str = r#"
struct VertexOut {
    @builtin(position) pos: vec4<f32>,
    @location(0)       uv:  vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOut {
    // Two triangles covering clip space
    var positions = array<vec2<f32>, 6>(
        vec2(-1.0, -1.0), vec2( 1.0, -1.0), vec2(-1.0,  1.0),
        vec2(-1.0,  1.0), vec2( 1.0, -1.0), vec2( 1.0,  1.0),
    );
    let p = positions[vi];
    var out: VertexOut;
    out.pos = vec4(p, 0.0, 1.0);
    out.uv  = p * 0.5 + 0.5;
    return out;
}

@group(0) @binding(0) var t_result:  texture_2d<f32>;
@group(0) @binding(1) var s_result:  sampler;

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    return textureSample(t_result, s_result, in.uv);
}
"#;
