struct VSOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VSOut {
    // Full-screen triangle without a vertex buffer
    // (-1,-1) (3,-1) (-1,3)
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    var out: VSOut;
    out.pos = vec4<f32>(pos[vi], 0.0, 1.0);
    // Map XY from [-1,1] to [0,1]
    out.uv = (pos[vi] * 0.5) + vec2<f32>(0.5, 0.5);
    return out;
}

@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var src_sampler: sampler;

struct Params {
    strength: f32,
    time: f32,
    aspect: f32,
    _pad: f32,
};
@group(0) @binding(2) var<uniform> params: Params;

@fragment
fn fs_main(in: VSOut) -> @location(0) vec4<f32> {
    let uv = in.uv;
    var color = textureSample(src_tex, src_sampler, uv);

    // Sample effect: vignette + slight color shift over time
    let center = vec2<f32>(0.5, 0.5);
    let dist = distance(uv, center);
    let vignette = 1.0 - smoothstep(0.3, 0.8, dist);
    let wobble = 0.03 * sin(params.time * 1.7 + uv.x * 10.0) * params.strength;

    color.rgb = color.rgb * (0.95 + 0.05 * sin(params.time * 0.9)) * vignette;
    color.r += wobble;
    return color;
}