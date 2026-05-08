#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::pbr_bindings
#import bevy_pbr::pbr_types

struct RayPayload {
    color: vec3<f32>,
    distance: f32,
    normal: vec3<f32>,
    metallic: f32,
    roughness: f32,
    emission: vec3<f32>,
};

struct Ray {
    origin: vec3<f32>,
    direction: vec3<f32>,
    t_min: f32,
    t_max: f32,
};

struct RayTracingSettings {
    max_bounces: u32,
    samples_per_pixel: u32,
    max_ray_distance: f32,
    ao_enabled: u32,
    ao_samples: u32,
    ao_radius: f32,
    caustics_enabled: u32,
    caustic_photons: u32,
    volumetric_enabled: u32,
    volumetric_resolution: vec3<f32>,
};

@group(2) @binding(0)
var acceleration_structure: acceleration_structure;

@group(2) @binding(1)
var output_texture: texture_storage_2d<rgba8unorm, write>;

@group(2) @binding(2)
var ao_buffer: texture_storage_2d<r32float, read_write>;

@group(2) @binding(3)
var caustics_buffer: texture_storage_2d<rgba32float, read_write>;

@group(2) @binding(4)
var volumetric_buffer: texture_storage_3d<rgba32float, read_write>;

@group(2) @binding(5)
var<uniform> settings: RayTracingSettings;

// Ray generation shader
@compute @workgroup_size(8, 8, 1)
fn ray_generation(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(workgroup_id) workgroup_id: vec3<u32>,
) {
    let dimensions = textureDimensions(output_texture);
    let pixel_coord = vec2<i32>(global_id.xy);
    
    if (pixel_coord.x >= dimensions.x || pixel_coord.y >= dimensions.y) {
        return;
    }

    var final_color = vec3<f32>(0.0);
    
    // Multi-sample anti-aliasing
    for (var sample = 0u; sample < settings.samples_per_pixel; sample++) {
        let jitter = get_sample_jitter(sample);
        let pixel_center = vec2<f32>(pixel_coord) + jitter;
        let ndc = (pixel_center / vec2<f32>(dimensions.xy)) * 2.0 - 1.0;
        
        var ray: Ray;
        ray.origin = view.world_position.xyz;
        ray.direction = normalize(calculate_ray_direction(ndc));
        ray.t_min = 0.001;
        ray.t_max = settings.max_ray_distance;

        // Trace primary ray
        let payload = trace_ray(ray, 0u);
        var color = calculate_lighting(payload);

        // Add ambient occlusion if enabled
        if (settings.ao_enabled != 0u) {
            let ao = calculate_ambient_occlusion(payload.world_position, payload.normal);
            color *= ao;
        }

        // Add caustics if enabled
        if (settings.caustics_enabled != 0u) {
            let caustics = calculate_caustics(payload.world_position);
            color += caustics;
        }

        // Add volumetric lighting if enabled
        if (settings.volumetric_enabled != 0u) {
            let volumetric = calculate_volumetric_lighting(ray);
            color = mix(color, volumetric, volumetric.a);
        }

        final_color += color;
    }

    final_color /= f32(settings.samples_per_pixel);
    
    // Store result
    textureStore(output_texture, pixel_coord, vec4<f32>(final_color, 1.0));
}

fn get_sample_jitter(sample_index: u32) -> vec2<f32> {
    // Halton sequence for quasi-random sampling
    let halton2 = halton_sequence(sample_index, 2u);
    let halton3 = halton_sequence(sample_index, 3u);
    return vec2<f32>(halton2, halton3);
}

fn halton_sequence(index: u32, base: u32) -> f32 {
    var f = 1.0;
    var r = 0.0;
    var i = index;
    
    while (i > 0u) {
        f = f / f32(base);
        r = r + f * f32(i % base);
        i = i / base;
    }
    
    return r;
}

fn calculate_ambient_occlusion(position: vec3<f32>, normal: vec3<f32>) -> f32 {
    var occlusion = 0.0;
    
    for (var i = 0u; i < settings.ao_samples; i++) {
        let sample_dir = sample_hemisphere(normal, i);
        var ao_ray: Ray;
        ao_ray.origin = position + normal * 0.001;
        ao_ray.direction = sample_dir;
        ao_ray.t_min = 0.001;
        ao_ray.t_max = settings.ao_radius;
        
        let hit = trace_visibility(ao_ray);
        if (!hit) {
            occlusion += 1.0;
        }
    }
    
    return 1.0 - (occlusion / f32(settings.ao_samples));
}

fn calculate_caustics(position: vec3<f32>) -> vec3<f32> {
    let uv = project_to_caustics_map(position);
    return textureSample(caustics_buffer, caustics_sampler, uv).rgb;
}

fn calculate_volumetric_lighting(ray: Ray) -> vec4<f32> {
    var result = vec4<f32>(0.0);
    let step_size = settings.max_ray_distance / 64.0;
    
    for (var t = ray.t_min; t < ray.t_max; t += step_size) {
        let pos = ray.origin + ray.direction * t;
        let uvw = world_to_volume_coords(pos);
        let sample = textureSample(volumetric_buffer, volumetric_sampler, uvw);
        
        // Front-to-back blending
        result.rgb += (1.0 - result.a) * sample.a * sample.rgb;
        result.a += (1.0 - result.a) * sample.a;
        
        if (result.a >= 0.99) {
            break;
        }
    }
    
    return result;
}

fn sample_hemisphere(normal: vec3<f32>, index: u32) -> vec3<f32> {
    let u1 = halton_sequence(index, 2u);
    let u2 = halton_sequence(index, 3u);
    
    let r = sqrt(1.0 - u1 * u1);
    let phi = 2.0 * 3.1415926535897932384626433832795 * u2;
    
    let x = cos(phi) * r;
    let y = sin(phi) * r;
    let z = u1;
    
    let up = abs(normal.z) < 0.999 ? vec3<f32>(0.0, 0.0, 1.0) : vec3<f32>(1.0, 0.0, 0.0);
    let tangent = normalize(cross(up, normal));
    let bitangent = cross(normal, tangent);
    
    return normalize(tangent * x + bitangent * y + normal * z);
}

fn project_to_caustics_map(position: vec3<f32>) -> vec2<f32> {
    // Project world position to caustics texture space
    // This is a simplified projection, should be adjusted based on scene setup
    return vec2<f32>(
        position.x / 10.0 + 0.5,
        position.z / 10.0 + 0.5
    );
}

fn world_to_volume_coords(position: vec3<f32>) -> vec3<f32> {
    // Transform world position to volumetric grid coordinates
    return (position - volume_min) / (volume_max - volume_min);
}

fn calculate_ray_direction(ndc: vec2<f32>) -> vec3<f32> {
    let view_matrix = view.view_proj;
    let inv_projection = view.inverse_projection;
    let inv_view = view.inverse_view;
    
    let target = inv_projection * vec4<f32>(ndc.x, ndc.y, 1.0, 1.0);
    let world_dir = inv_view * vec4<f32>(normalize(target.xyz), 0.0);
    
    return normalize(world_dir.xyz);
}

fn trace_ray(ray: Ray) -> RayPayload {
    var payload: RayPayload;
    payload.color = vec3<f32>(0.0);
    payload.distance = ray.t_max;
    payload.normal = vec3<f32>(0.0, 1.0, 0.0);
    payload.metallic = 0.0;
    payload.roughness = 1.0;
    payload.emission = vec3<f32>(0.0);

    let flags = RAY_FLAG_FORCE_OPAQUE | RAY_FLAG_CULL_BACK_FACING_TRIANGLES;
    
    // Trace ray through acceleration structure
    traceRay(
        acceleration_structure,
        flags,
        0xFF, // Instance mask
        0,    // Ray type (primary)
        1,    // Number of ray types
        0,    // Miss shader index
        ray.origin,
        ray.t_min,
        ray.direction,
        ray.t_max,
        0     // Payload location
    );

    return payload;
}

fn calculate_lighting(payload: RayPayload) -> vec3<f32> {
    if (payload.distance >= RAY_MAX_DISTANCE) {
        return vec3<f32>(0.0); // Sky color or environment map could be used here
    }

    // Create PBR input from ray traced data
    var pbr_input: PbrInput;
    pbr_input.material.base_color = vec4<f32>(payload.color, 1.0);
    pbr_input.material.metallic = payload.metallic;
    pbr_input.material.perceptual_roughness = payload.roughness;
    pbr_input.material.reflectance = 0.5;
    pbr_input.material.emissive = payload.emission;
    pbr_input.world_position = vec4<f32>(payload.origin + payload.direction * payload.distance, 1.0);
    pbr_input.world_normal = payload.normal;
    pbr_input.is_orthographic = false;

    // Calculate PBR lighting
    let pbr_output = pbr(pbr_input);
    
    return pbr_output.color;
}

// Ray hit shader
@raytracing
fn closest_hit(
    @builtin(ray_tmax) ray_tmax: f32,
    @builtin(primitive_id) primitive_id: u32,
    @builtin(instance_id) instance_id: u32,
    @builtin(ray_world_origin) ray_origin: vec3<f32>,
    @builtin(ray_world_direction) ray_direction: vec3<f32>,
    @builtin(ray_barycentrics) barycentrics: vec2<f32>,
) {
    // Get hit point information
    let hit_point = ray_origin + ray_direction * ray_tmax;
    let normal = calculate_hit_normal(primitive_id, barycentrics);
    
    // Sample material properties at hit point
    let material = get_material_properties(instance_id, primitive_id, barycentrics);
    
    // Store in payload
    payload.color = material.base_color;
    payload.distance = ray_tmax;
    payload.normal = normal;
    payload.metallic = material.metallic;
    payload.roughness = material.roughness;
    payload.emission = material.emission;
}

// Ray miss shader
@raytracing
fn miss(
    @builtin(ray_world_origin) ray_origin: vec3<f32>,
    @builtin(ray_world_direction) ray_direction: vec3<f32>,
) {
    payload.distance = RAY_MAX_DISTANCE;
} 