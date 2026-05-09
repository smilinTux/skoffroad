// Post-process tier — Sprint 41 commit 7.
//
// Adds a quality-gated layer of post-processing on top of the camera:
//   - filmic tonemap (AgX) on Medium+
//   - subtle ColorGrading bump on Medium+ (contrast + saturation)
//   - SSAO on High only (depth + normal prepasses are auto-required)
//
// Bloom is owned by `bloom_pp` already; this plugin doesn't touch it. We
// run in PostStartup to attach to the Camera3d entity that camera.rs
// spawned during Startup.

use bevy::core_pipeline::tonemapping::Tonemapping;
#[cfg(not(target_arch = "wasm32"))]
use bevy::pbr::ScreenSpaceAmbientOcclusion;
#[cfg(not(target_arch = "wasm32"))]
use bevy::pbr::ScreenSpaceAmbientOcclusionQualityLevel;
use bevy::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use bevy::render::view::Msaa;
use bevy::render::view::ColorGrading;

use crate::graphics_quality::GraphicsQuality;

pub struct PostFxPlugin;

impl Plugin for PostFxPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, attach_post_fx);
    }
}

fn attach_post_fx(
    mut commands: Commands,
    quality: Res<GraphicsQuality>,
    cameras: Query<Entity, With<Camera3d>>,
) {
    let Ok(cam) = cameras.single() else {
        return;
    };

    if quality.filmic_tonemap() {
        // AgX: very neutral, almost no hue shifting. No LUT feature required.
        commands.entity(cam).insert(Tonemapping::AgX);

        // Subtle filmic feel: post_saturation lifts colour vibrancy across the
        // whole image; per-section contrast adds bite to shadows and lets
        // highlights breathe. Numbers picked to be noticeable but not gaudy.
        let mut grading = ColorGrading::default();
        grading.global.exposure        = 0.0;
        grading.global.post_saturation = 1.06;
        grading.shadows.contrast       = 1.05;
        grading.midtones.contrast      = 1.03;
        grading.highlights.contrast    = 1.02;
        commands.entity(cam).insert(grading);
    }

    if quality.ssao() {
        // SSAO at "Low" preset is the right perf/quality knee for an
        // open-world driving game. Medium would halve frame budget.
        // SSAO is incompatible with MSAA — turn it off on this camera so
        // bevy_pbr::ssao doesn't spam validation errors. The Sample4
        // default is restored automatically on lower tiers.
        //
        // WASM/WebGL2: SSAO is explicitly unsupported (requires compute
        // storage textures that WebGL2 lacks). Its depth prepass triggers
        // hundreds of GL_INVALID_FRAMEBUFFER_OPERATION errors from wgpu's
        // WebGL2 backend when it tries to copy depth textures. Skip entirely
        // on wasm32; default quality will read High but we silently drop SSAO.
        #[cfg(not(target_arch = "wasm32"))]
        commands.entity(cam).insert((
            ScreenSpaceAmbientOcclusion {
                quality_level: ScreenSpaceAmbientOcclusionQualityLevel::Low,
                constant_object_thickness: 0.25,
            },
            Msaa::Off,
        ));
    }

    info!(
        "post_fx: tier={} -> tonemap={}, ssao={}",
        quality.as_str(),
        quality.filmic_tonemap(),
        quality.ssao(),
    );
}
