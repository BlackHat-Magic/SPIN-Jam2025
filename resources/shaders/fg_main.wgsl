struct FragmentInput {
    @location(0) worldPos : vec3<f32>,
    @location(1) normal   : vec3<f32>,
    @location(2) uv       : vec2<f32>,
};

struct Light {
    position : vec3<f32>,
    _pad1    : f32,
    color    : vec3<f32>,
    _pad2    : f32,
};

@group(0) @binding(1) var<storage, read> lights : array<Light>;
@group(0) @binding(2) var<uniform> cameraPos : vec3<f32>;

@group(0) @binding(3) var albedo_tex: texture_2d<f32>;
@group(0) @binding(4) var albedo_sampler: sampler;
@group(0) @binding(5) var metallic_tex: texture_2d<f32>;
@group(0) @binding(6) var metallic_sampler: sampler;
@group(0) @binding(7) var roughness_tex: texture_2d<f32>;
@group(0) @binding(8) var roughness_sampler: sampler;
@group(0) @binding(9) var ao_tex: texture_2d<f32>;
@group(0) @binding(10) var ao_sampler: sampler;

fn fresnelSchlick(cosTheta: f32, F0: vec3<f32>) -> vec3<f32> {
    return F0 + (1.0 - F0) * pow(1.0 - cosTheta, 5.0);
}

fn distributionGGX(N: vec3<f32>, H: vec3<f32>, roughness: f32) -> f32 {
    let a      = roughness * roughness;
    let a2     = a * a;
    let NdotH  = max(dot(N, H), 0.0);
    let NdotH2 = NdotH * NdotH;

    let denom = NdotH2 * (a2 - 1.0) + 1.0;
    return a2 / (3.14159265 * denom * denom);
}

fn geometrySchlickGGX(NdotV: f32, roughness: f32) -> f32 {
    let r = roughness + 1.0;
    let k = (r * r) / 8.0;
    return NdotV / (NdotV * (1.0 - k) + k);
}

fn geometrySmith(N: vec3<f32>, V: vec3<f32>, L: vec3<f32>, roughness: f32) -> f32 {
    let NdotV = max(dot(N, V), 0.0);
    let NdotL = max(dot(N, L), 0.0);
    let ggx1 = geometrySchlickGGX(NdotV, roughness);
    let ggx2 = geometrySchlickGGX(NdotL, roughness);
    return ggx1 * ggx2;
}

@fragment
fn main(input: FragmentInput) -> @location(0) vec4<f32> {
    let N = normalize(input.normal);
    let V = normalize(cameraPos - input.worldPos);

    // Sample textures once
    let albedo = textureSample(albedo_tex, albedo_sampler, input.uv).rgb;
    let metallic = textureSample(metallic_tex, metallic_sampler, input.uv).r;
    let roughness = textureSample(roughness_tex, roughness_sampler, input.uv).r;
    let ao = textureSample(ao_tex, ao_sampler, input.uv).r;

    let F0 = mix(vec3<f32>(0.04), albedo, metallic);

    var Lo = vec3<f32>(0.0);
    for (var i = 0u; i < arrayLength(&lights); i = i + 1u) {
        let light = lights[i];
        let L = normalize(light.position - input.worldPos);
        let H = normalize(V + L);

        let distance = length(light.position - input.worldPos);
        let attenuation = 1.0 / (distance * distance);
        let radiance = light.color * attenuation;

        let NDF = distributionGGX(N, H, roughness);
        let G   = geometrySmith(N, V, L, roughness);
        let F   = fresnelSchlick(max(dot(H, V), 0.0), F0);

        let numerator = NDF * G * F;
        let denominator = 4.0 * max(dot(N, V), 0.0) * max(dot(N, L), 0.0) + 0.001;
        let specular = numerator / denominator;

        let kS = F;
        var kD = vec3<f32>(1.0) - kS;
        kD *= 1.0 - metallic;

        let NdotL = max(dot(N, L), 0.0);
        Lo += (kD * albedo / 3.14159265 + specular) * radiance * NdotL;
    }

    let ambient = vec3<f32>(0.01) * albedo * ao;
    let color = ambient + Lo;

    let gamma = 2.2;
    let mapped = pow(color, vec3<f32>(1.0 / gamma));
    return vec4<f32>(mapped, 1.0);
}
/*@fragment
fn main(input: FragmentInput) -> @location(0) vec4<f32> {
    let N = normalize(input.normal);
    let V = normalize(cameraPos - input.worldPos);
    let L = normalize(light.position - input.worldPos);
    let H = normalize(V + L);

    let distance    = length(light.position - input.worldPos);
    let attenuation = 1.0 / (distance * distance);
    let radiance    = light.color * attenuation;

    // Sample textures
    let albedo = textureSample(albedo_tex, albedo_sampler, input.uv).rgb;
    let metallic = textureSample(metallic_tex, metallic_sampler, input.uv).r;
    let roughness = textureSample(roughness_tex, roughness_sampler, input.uv).r;
    let ao = textureSample(ao_tex, ao_sampler, input.uv).r;

    let F0 = mix(vec3<f32>(0.04, 0.04, 0.04), albedo, metallic);

    let NDF = distributionGGX(N, H, roughness);
    let G   = geometrySmith(N, V, L, roughness);
    let F   = fresnelSchlick(max(dot(H, V), 0.0), F0);

    let numerator    = NDF * G * F;
    let denominator  = 4.0 * max(dot(N, V), 0.0) * max(dot(N, L), 0.0) + 0.001;
    let specular     = numerator / denominator;

    let kS = F;
    var kD = vec3<f32>(1.0, 1.0, 1.0) - kS;
    kD *= 1.0 - metallic;

    let NdotL = max(dot(N, L), 0.0);

    let Lo = (kD * albedo / 3.14159265 + specular) * radiance * NdotL;

    let ambient = vec3<f32>(0.01, 0.01, 0.01) * albedo * ao;

    let color = ambient + Lo;

    // Gamma correction
    let gamma = 2.2;
    let mapped = pow(color, vec3<f32>(1.0 / gamma));

    return vec4<f32>(mapped, 1.0);
}*/
