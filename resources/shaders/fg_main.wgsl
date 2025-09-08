struct FragmentInput {
    @location(0) worldPos : vec3<f32>,
    @location(1) normal   : vec3<f32>,
    @location(2) uv       : vec2<f32>,
};

struct Material {
    albedo    : vec3<f32>,
    metallic  : f32,
    roughness : f32,
    ao        : f32,
};

struct Light {
    position : vec3<f32>,
    color    : vec3<f32>,
};

@group(0) @binding(1) var<uniform> light : Light;
@group(0) @binding(2) var<uniform> cameraPos : vec3<f32>;
@group(0) @binding(3) var<uniform> material : Material;

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
    let L = normalize(light.position - input.worldPos);
    let H = normalize(V + L);

    let distance    = length(light.position - input.worldPos);
    let attenuation = 1.0 / (distance * distance);
    let radiance    = light.color * attenuation;

    let albedo    = material.albedo;
    let metallic  = material.metallic;
    let roughness = material.roughness;
    let ao        = material.ao;

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

    let ambient = vec3<f32>(0.03, 0.03, 0.03) * albedo * ao;

    let color = ambient + Lo;

    // Gamma correction
    let gamma = 2.2;
    let mapped = pow(color, vec3<f32>(1.0 / gamma));

    return vec4<f32>(mapped, 1.0);
}