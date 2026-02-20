// Effect: map raw escape-time value (r channel) → RGB colour.
// Scheme is encoded in the uniforms as an integer:
//   0 = Classic, 1 = Fire, 2 = Ocean, 3 = Psychedelic

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
struct EffectParams {
    scheme : u32,
    _pad0  : u32,
    _pad1  : u32,
    _pad2  : u32,
}

@group(0) @binding(0) var<uniform>  u      : Uniforms;
@group(0) @binding(1) var<uniform>  ep     : EffectParams;
@group(0) @binding(2) var           input  : texture_2d<f32>;
@group(0) @binding(3) var           output : texture_storage_2d<rgba16float, write>;

fn classic(t: f32) -> vec3<f32> {
    return 0.5 + 0.5 * vec3(cos(TAU * (t + 0.0)),
                             cos(TAU * (t + 0.33)),
                             cos(TAU * (t + 0.67)));
}
fn fire(t: f32) -> vec3<f32> {
    return vec3(t, t * t, t * t * t);
}
fn ocean(t: f32) -> vec3<f32> {
    return vec3(0.0, t * 0.5, t);
}
fn psychedelic(t: f32) -> vec3<f32> {
    return 0.5 + 0.5 * vec3(sin(t * 30.0), sin(t * 19.0 + 1.0), sin(t * 13.0 + 2.0));
}

const TAU: f32 = 6.28318530718;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let coord = vec2<i32>(gid.xy);
    let px    = textureLoad(input, coord, 0);
    let t     = px.r; // normalised escape value in [0, 1]

    var rgb: vec3<f32>;
    switch ep.scheme {
        case 1u:  { rgb = fire(t); }
        case 2u:  { rgb = ocean(t); }
        case 3u:  { rgb = psychedelic(t); }
        default:  { rgb = classic(t); }
    }

    textureStore(output, coord, vec4<f32>(rgb, 1.0));
}
