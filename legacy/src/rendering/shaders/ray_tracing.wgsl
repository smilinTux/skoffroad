// Ray tracing shader with advanced material features

struct RayTracingSettings {
    max_bounces: u32,
    samples_per_pixel: u32,
    subsurface_enabled: u32,
    subsurface_samples: u32,
    subsurface_radius: f32,
    dispersion_enabled: u32,
    dispersion_samples: u32,
    dispersion_range: vec2<f32>,
};

@group(0) @binding(0) var<uniform> settings: RayTracingSettings;
@group(0) @binding(1) var<storage> acceleration_structure: AccelerationStructure;
@group(0) @binding(2) var output_texture: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(3) var<storage> material_buffer: MaterialBuffer;
@group(0) @binding(4) var<storage> light_buffer: LightBuffer;
@group(0) @binding(5) var<storage> subsurface_buffer: SubsurfaceBuffer;
@group(0) @binding(6) var<storage> dispersion_buffer: DispersionBuffer;

struct Ray {
    origin: vec3<f32>,
    direction: vec3<f32>,
    wavelength: f32,  // For dispersion
};

struct HitInfo {
    position: vec3<f32>,
    normal: vec3<f32>,
    material_id: u32,
    distance: f32,
};

// Fresnel equations for dispersion
fn calculate_fresnel_dispersion(n1: f32, n2: f32, cos_i: f32) -> f32 {
    let r0 = ((n1 - n2) / (n1 + n2)) * ((n1 - n2) / (n1 + n2));
    return r0 + (1.0 - r0) * pow(1.0 - cos_i, 5.0);
}

// Calculate refraction direction with wavelength-dependent IOR
fn refract_ray(incident: vec3<f32>, normal: vec3<f32>, n1: f32, n2: f32) -> vec3<f32> {
    let cos_i = -dot(normal, incident);
    let n = n1 / n2;
    let cos_t2 = 1.0 - n * n * (1.0 - cos_i * cos_i);
    
    if cos_t2 < 0.0 {
        return vec3<f32>(0.0);  // Total internal reflection
    }
    
    return n * incident + (n * cos_i - sqrt(cos_t2)) * normal;
}

// Subsurface scattering calculation
fn calculate_subsurface_scattering(hit: HitInfo, material: Material) -> vec3<f32> {
    if settings.subsurface_enabled == 0u {
        return vec3<f32>(0.0);
    }

    var result = vec3<f32>(0.0);
    let radius = settings.subsurface_radius;
    
    for(var i = 0u; i < settings.subsurface_samples; i = i + 1u) {
        // Generate sample points in a sphere around hit point
        let theta = random_float() * 2.0 * 3.14159;
        let phi = acos(2.0 * random_float() - 1.0);
        let r = radius * pow(random_float(), 1.0/3.0);
        
        let sample_offset = vec3<f32>(
            r * sin(phi) * cos(theta),
            r * sin(phi) * sin(theta),
            r * cos(phi)
        );
        
        let sample_point = hit.position + sample_offset;
        
        // Calculate scattering based on material properties
        let scatter_distance = length(sample_offset);
        let attenuation = exp(-scatter_distance / material.scatter_distance);
        
        // Trace ray from sample point to light sources
        result += trace_to_lights(sample_point, hit.normal) * attenuation;
    }
    
    return result / f32(settings.subsurface_samples);
}

// Dispersion calculation
fn calculate_dispersion(ray: Ray, hit: HitInfo, material: Material) -> vec3<f32> {
    if settings.dispersion_enabled == 0u {
        return vec3<f32>(0.0);
    }

    var result = vec3<f32>(0.0);
    let wavelength_range = settings.dispersion_range;
    
    for(var i = 0u; i < settings.dispersion_samples; i = i + 1u) {
        let t = f32(i) / f32(settings.dispersion_samples - 1u);
        let wavelength = mix(wavelength_range.x, wavelength_range.y, t);
        
        // Calculate wavelength-dependent IOR
        let ior = material.base_ior + material.dispersion_strength * (1.0 / wavelength - 1.0 / wavelength_range.x);
        
        // Create dispersed ray
        var dispersed_ray: Ray;
        dispersed_ray.origin = hit.position + hit.normal * 0.001;  // Offset to avoid self-intersection
        dispersed_ray.direction = refract_ray(ray.direction, hit.normal, 1.0, ior);
        dispersed_ray.wavelength = wavelength;
        
        // Trace dispersed ray
        let color = trace_ray(dispersed_ray, settings.max_bounces);
        
        // Convert wavelength to RGB
        result += wavelength_to_rgb(wavelength) * color;
    }
    
    return result / f32(settings.dispersion_samples);
}

// Main ray tracing function
@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let dimensions = textureDimensions(output_texture);
    if global_id.x >= dimensions.x || global_id.y >= dimensions.y {
        return;
    }

    var pixel_color = vec3<f32>(0.0);
    
    for(var s = 0u; s < settings.samples_per_pixel; s = s + 1u) {
        let ray = generate_camera_ray(global_id.xy, dimensions);
        var result = trace_ray(ray, settings.max_bounces);
        
        if let Some(hit) = trace_ray(ray) {
            let material = get_material(hit.material_id);
            
            // Add subsurface scattering
            if material.subsurface_scatter > 0.0 {
                result += calculate_subsurface_scattering(hit, material);
            }
            
            // Add dispersion
            if material.dispersion_strength > 0.0 {
                result += calculate_dispersion(ray, hit, material);
            }
        }
        
        pixel_color += result;
    }
    
    pixel_color = pixel_color / f32(settings.samples_per_pixel);
    
    // Tone mapping and gamma correction
    pixel_color = tone_map(pixel_color);
    pixel_color = pow(pixel_color, vec3<f32>(1.0/2.2));
    
    textureStore(output_texture, global_id.xy, vec4<f32>(pixel_color, 1.0));
} 