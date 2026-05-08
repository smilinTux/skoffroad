// Color utility functions for ray tracing

// Convert wavelength (in nanometers) to RGB color
fn wavelength_to_rgb(wavelength: f32) -> vec3<f32> {
    // Wavelength must be between 380 and 750 nm
    var r = 0.0;
    var g = 0.0;
    var b = 0.0;
    
    if wavelength >= 380.0 && wavelength < 440.0 {
        r = -(wavelength - 440.0) / (440.0 - 380.0);
        b = 1.0;
    } else if wavelength >= 440.0 && wavelength < 490.0 {
        g = (wavelength - 440.0) / (490.0 - 440.0);
        b = 1.0;
    } else if wavelength >= 490.0 && wavelength < 510.0 {
        g = 1.0;
        b = -(wavelength - 510.0) / (510.0 - 490.0);
    } else if wavelength >= 510.0 && wavelength < 580.0 {
        r = (wavelength - 510.0) / (580.0 - 510.0);
        g = 1.0;
    } else if wavelength >= 580.0 && wavelength < 645.0 {
        r = 1.0;
        g = -(wavelength - 645.0) / (645.0 - 580.0);
    } else if wavelength >= 645.0 && wavelength <= 750.0 {
        r = 1.0;
    }
    
    // Intensity falloff at spectrum edges
    let factor = 1.0;
    if wavelength > 700.0 {
        factor = 0.3 + 0.7 * (750.0 - wavelength) / (750.0 - 700.0);
    } else if wavelength < 420.0 {
        factor = 0.3 + 0.7 * (wavelength - 380.0) / (420.0 - 380.0);
    }
    
    return vec3<f32>(r, g, b) * factor;
}

// ACES tone mapping operator
fn tone_map(color: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((color * (a * color + b)) / (color * (c * color + d) + e), vec3<f32>(0.0), vec3<f32>(1.0));
}

// Convert RGB to luminance
fn rgb_to_luminance(color: vec3<f32>) -> f32 {
    return dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
}

// Convert RGB to HSV
fn rgb_to_hsv(color: vec3<f32>) -> vec3<f32> {
    let K = vec4<f32>(0.0, -1.0/3.0, 2.0/3.0, -1.0);
    let p = mix(vec4<f32>(color.bg, K.wz), vec4<f32>(color.gb, K.xy), step(color.b, color.g));
    let q = mix(vec4<f32>(p.xyw, color.r), vec4<f32>(color.r, p.yzx), step(p.x, color.r));
    
    let d = q.x - min(q.w, q.y);
    let e = 1.0e-10;
    return vec3<f32>(
        abs(q.z + (q.w - q.y) / (6.0 * d + e)),
        d / (q.x + e),
        q.x
    );
}

// Convert HSV to RGB
fn hsv_to_rgb(hsv: vec3<f32>) -> vec3<f32> {
    let K = vec4<f32>(1.0, 2.0/3.0, 1.0/3.0, 3.0);
    let p = abs(fract(hsv.xxx + K.xyz) * 6.0 - K.www);
    return hsv.z * mix(K.xxx, clamp(p - K.xxx, vec3<f32>(0.0), vec3<f32>(1.0)), hsv.y);
} 