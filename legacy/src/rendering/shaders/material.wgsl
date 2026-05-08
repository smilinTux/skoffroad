// Material definitions and functions for PBR rendering

struct Material {
    // Base material properties
    albedo: vec3<f32>,
    metallic: f32,
    roughness: f32,
    ior: f32,  // Index of refraction
    
    // Advanced material properties
    emission: vec3<f32>,
    emission_strength: f32,
    alpha: f32,  // Transparency
    
    // Subsurface scattering properties
    subsurface_radius: vec3<f32>,  // RGB radius for SSS
    subsurface_strength: f32,
    
    // Dispersion properties
    dispersion_strength: f32,  // Abbe number inverse
    dispersion_bias: f32,      // Wavelength bias
    
    // Flags
    use_subsurface: u32,
    use_dispersion: u32,
};

// Calculate Fresnel reflectance using IOR
fn fresnel_schlick(cos_theta: f32, f0: vec3<f32>) -> vec3<f32> {
    return f0 + (vec3<f32>(1.0) - f0) * pow(1.0 - cos_theta, 5.0);
}

// Calculate normal distribution function (GGX/Trowbridge-Reitz)
fn distribution_ggx(n_dot_h: f32, roughness: f32) -> f32 {
    let alpha = roughness * roughness;
    let alpha2 = alpha * alpha;
    let denom = n_dot_h * n_dot_h * (alpha2 - 1.0) + 1.0;
    return alpha2 / (PI * denom * denom);
}

// Calculate geometry function (Smith GGX)
fn geometry_smith(n_dot_v: f32, n_dot_l: f32, roughness: f32) -> f32 {
    let r = roughness + 1.0;
    let k = (r * r) / 8.0;
    let ggx1 = n_dot_v / (n_dot_v * (1.0 - k) + k);
    let ggx2 = n_dot_l / (n_dot_l * (1.0 - k) + k);
    return ggx1 * ggx2;
}

// Calculate refraction direction with dispersion
fn calc_refraction(incident: vec3<f32>, normal: vec3<f32>, ior: f32, wavelength: f32, material: Material) -> vec3<f32> {
    // Calculate wavelength-dependent IOR using Cauchy's equation
    let base_ior = ior;
    let dispersion = material.dispersion_strength * 0.01;  // Scale factor for dispersion
    let wavelength_bias = material.dispersion_bias;
    
    // Cauchy's equation: n(λ) = A + B/λ^2
    let wavelength_um = wavelength * 0.001;  // Convert nm to μm
    let actual_ior = base_ior + dispersion * (1.0 / (wavelength_um * wavelength_um)) + wavelength_bias;
    
    let cos_i = dot(-incident, normal);
    let sin_t2 = (1.0 - cos_i * cos_i) / (actual_ior * actual_ior);
    
    if sin_t2 > 1.0 {
        // Total internal reflection
        return reflect(incident, normal);
    }
    
    let cos_t = sqrt(1.0 - sin_t2);
    return incident * (1.0 / actual_ior) + normal * (cos_i / actual_ior - cos_t);
}

// Calculate subsurface scattering contribution
fn calc_subsurface(position: vec3<f32>, normal: vec3<f32>, material: Material) -> vec3<f32> {
    var result = vec3<f32>(0.0);
    let samples = 8u;  // Number of subsurface samples
    
    // Generate sample points in a sphere around the hit point
    for (var i = 0u; i < samples; i = i + 1u) {
        let theta = (f32(i) / f32(samples)) * 2.0 * PI;
        let phi = acos(2.0 * random() - 1.0);
        
        let sample_dir = vec3<f32>(
            sin(phi) * cos(theta),
            sin(phi) * sin(theta),
            cos(phi)
        );
        
        // Scale by material's subsurface radius
        let sample_pos = position + sample_dir * material.subsurface_radius;
        
        // TODO: Add actual light sampling and scattering calculation here
        // For now, we'll use a simple approximation
        let scatter_distance = length(sample_pos - position);
        let scatter_factor = exp(-scatter_distance / material.subsurface_strength);
        
        result += material.albedo * scatter_factor;
    }
    
    return result / f32(samples);
} 