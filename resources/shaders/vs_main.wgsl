struct VertexInput {
    @location(0) position : vec3<f32>,
    @location(1) normal   : vec3<f32>,
    @location(2) uv       : vec2<f32>,
};

struct Uniforms {
    model      : mat4x4<f32>,
    view       : mat4x4<f32>,
    projection : mat4x4<f32>,
};

struct VertexOutput {
    @builtin(position) position : vec4<f32>,
    @location(0) worldPos       : vec3<f32>,
    @location(1) normal         : vec3<f32>,
    @location(2) uv             : vec2<f32>,
};

@group(0) @binding(0) var<uniform> uniforms : Uniforms;

@vertex
fn main(input : VertexInput) -> VertexOutput {
    var output : VertexOutput;
    let worldPos = (uniforms.model * vec4<f32>(input.position, 1.0)).xyz;
    output.position = uniforms.projection * uniforms.view * vec4<f32>(worldPos, 1.0);
    output.worldPos = worldPos;
    output.normal = normalize((uniforms.model * vec4<f32>(input.normal, 0.0)).xyz);
    output.uv = input.uv;
    return output;
}