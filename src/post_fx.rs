// Post-process tier — Sprint 41 commit 7, expanded in Sprint 75.
//
// Quality-gated post-processing stack attached to the Camera3d entity:
//
//   Low    — Tonemapping::AgX only (all heavy effects off; safe for mobile/WASM)
//   Medium — + FXAA (cheap anti-aliasing, WebGL2-safe)
//   High   — + SSAO (native only) + DoF Gaussian (subtle) + upgraded shadows
//             + TAA on native (replaces FXAA at High to pair with SSAO)
//             + FXAA on WASM as TAA fallback
//
// Bloom is owned by `bloom_pp`; this plugin does not touch it.
//
// Runs in PostStartup (camera components) + PostUpdate run-once (tonemapping
// pin + shadow upgrade).  The PostUpdate system re-applies AgX tonemapping
// because bloom_pp.rs inserts AcesFitted in its first Update pass.
//
// ── WASM caveats ────────────────────────────────────────────────────────────
// SSAO needs compute storage textures (WebGPU / native only) → cfg-gated.
// TAA  needs compute → cfg-gated (FXAA fallback on WASM at High).
// DoF  uses Gaussian mode (Bokeh requires WebGPU) → works on WebGL2 + native.
// FXAA runs on WebGL2.
// Shadow map / cascade config is pure CPU state → safe on WASM.
//
// ── Tonemapping choice: AgX ──────────────────────────────────────────────────
// AgX is neutral and filmic with very little hue-shifting.  Compared to
// AcesFitted (which skews highlights orange) and TonyMcMapface (excellent
// but requires the `tonemapping_luts` feature), AgX gives the most natural
// outdoor daylight look for rocky terrain without extra feature flags.
// Applied at all tiers (essentially free: just a shader def switch).

use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::prelude::*;
use bevy::render::view::ColorGrading;

// SSAO — native only (requires compute storage textures)
#[cfg(not(target_arch = "wasm32"))]
use bevy::pbr::{ScreenSpaceAmbientOcclusion, ScreenSpaceAmbientOcclusionQualityLevel};
#[cfg(not(target_arch = "wasm32"))]
use bevy::render::view::Msaa;

// FXAA — works on WebGL2 and native
use bevy::anti_alias::fxaa::{Fxaa, Sensitivity};

// TAA — native only (needs compute; not available on WebGL2)
#[cfg(not(target_arch = "wasm32"))]
use bevy::anti_alias::taa::TemporalAntiAliasing;

// Depth of Field — Gaussian mode is WebGL2-safe; only High+ uses it
use bevy::post_process::dof::{DepthOfField, DepthOfFieldMode};

// Shadow config — DirectionalLightShadowMap is a Resource
use bevy::light::{CascadeShadowConfigBuilder, DirectionalLightShadowMap};

use crate::graphics_quality::GraphicsQuality;

pub struct PostFxPlugin;

impl Plugin for PostFxPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(PostStartup, attach_post_fx)
            // pin_tonemapping runs once in PostUpdate so it fires after
            // bloom_pp.rs's first-frame Update (which inserts AcesFitted).
            // run_once from bevy::prelude::common_conditions is re-exported
            // via bevy::prelude::*.
            .add_systems(PostUpdate, pin_tonemapping_and_shadows.run_if(run_once));
    }
}

// ---------------------------------------------------------------------------
// PostStartup: attach per-camera post-FX components
// ---------------------------------------------------------------------------

fn attach_post_fx(
    mut commands: Commands,
    quality: Res<GraphicsQuality>,
    cameras: Query<Entity, With<Camera3d>>,
) {
    let Ok(cam) = cameras.single() else {
        return;
    };

    // ── Tonemapping (all tiers) ─────────────────────────────────────────────
    // This is overridden by bloom_pp in the first Update frame; the
    // pin_tonemapping_and_shadows system below re-applies it in PostUpdate.
    commands.entity(cam).insert(Tonemapping::AgX);

    // Subtle filmic colour grading at Medium+ (Low keeps plain AgX).
    if *quality != GraphicsQuality::Low {
        let mut grading = ColorGrading::default();
        grading.global.exposure        = 0.0;
        grading.global.post_saturation = 1.06;
        grading.shadows.contrast       = 1.05;
        grading.midtones.contrast      = 1.03;
        grading.highlights.contrast    = 1.02;
        commands.entity(cam).insert(grading);
    }

    // ── Anti-aliasing ───────────────────────────────────────────────────────
    // Low  : off
    // Med  : FXAA (WebGL2-safe, cheap)
    // High : TAA on native (pairs well with SSAO; requires Msaa::Off)
    //        FXAA on WASM  (TAA not available on WebGL2)
    match *quality {
        GraphicsQuality::Low => {}
        GraphicsQuality::Medium => {
            commands.entity(cam).insert(Fxaa {
                enabled: true,
                edge_threshold: Sensitivity::High,
                edge_threshold_min: Sensitivity::Low,
            });
        }
        GraphicsQuality::High => {
            // --- native: TAA ------------------------------------------------
            #[cfg(not(target_arch = "wasm32"))]
            {
                // #[require] on TemporalAntiAliasing auto-inserts:
                //   TemporalJitter, MipBias, DepthPrepass, MotionVectorPrepass
                // TAA explicitly requires Msaa::Off.
                commands.entity(cam).insert((
                    TemporalAntiAliasing::default(),
                    Msaa::Off,
                ));
            }
            // --- WASM: FXAA fallback ----------------------------------------
            #[cfg(target_arch = "wasm32")]
            {
                commands.entity(cam).insert(Fxaa {
                    enabled: true,
                    edge_threshold: Sensitivity::High,
                    edge_threshold_min: Sensitivity::Low,
                });
            }
        }
    }

    // ── SSAO — High, native only ─────────────────────────────────────────────
    // Adds contact darkening under the truck, in rock crevices, at prop bases.
    // Medium preset (8 spp) is a good perf/quality knee for an open-world game.
    // Low (4 spp) has visible banding; High (18 spp) is too expensive.
    // WASM: WebGL2 lacks compute storage textures — skipped entirely.
    // #[require] on ScreenSpaceAmbientOcclusion auto-inserts DepthPrepass +
    // NormalPrepass.  Msaa is already Off from TAA above.
    #[cfg(not(target_arch = "wasm32"))]
    if *quality == GraphicsQuality::High {
        commands.entity(cam).insert(ScreenSpaceAmbientOcclusion {
            quality_level: ScreenSpaceAmbientOcclusionQualityLevel::Medium,
            constant_object_thickness: 0.25,
        });
    }

    // ── Depth of Field — High only, SUBTLE ───────────────────────────────────
    // Focal point at 35 m (gameplay area in front of the truck stays sharp).
    // f/22 keeps everything within ~10–80 m in focus; only distant mountains
    // (> 150 m) show a gentle softening.  Gaussian mode for WebGL2 compatibility.
    // Bokeh is richer but requires WebGPU compute — not used here.
    if *quality == GraphicsQuality::High {
        commands.entity(cam).insert(DepthOfField {
            mode: DepthOfFieldMode::Gaussian,
            focal_distance: 35.0,     // metres to the focus plane
            sensor_height: 0.018_66,  // Super-35 sensor (Bevy default)
            aperture_f_stops: 22.0,   // stopped-down → wide DoF, gentle blur
            max_circle_of_confusion_diameter: 32.0,
            max_depth: 800.0,         // cap sky/SkyDome infinite depth at 800 m
        });
    }

    // ── Shadow map resolution — High only ────────────────────────────────────
    // Raise the DirectionalLightShadowMap size from the 512-px default to
    // 2048 px so close-up rocks and the truck chassis get sharp shadow edges.
    // The DirectionalLight itself (shadows_enabled: false in sky.rs) is patched
    // by the PostUpdate system below so we can safely mutate it after sky.rs
    // runs its Startup systems.
    if *quality == GraphicsQuality::High {
        commands.insert_resource(DirectionalLightShadowMap { size: 2048 });
    }

    info!(
        "post_fx: tier={} -> tonemap=AgX grading={} fxaa={} ssao={} taa={} dof={} hi_shadows={}",
        quality.as_str(),
        *quality != GraphicsQuality::Low,
        matches!(*quality, GraphicsQuality::Medium)
            || (*quality == GraphicsQuality::High && cfg!(target_arch = "wasm32")),
        *quality == GraphicsQuality::High,
        *quality == GraphicsQuality::High && !cfg!(target_arch = "wasm32"),
        *quality == GraphicsQuality::High,
        *quality == GraphicsQuality::High,
    );
}

// ---------------------------------------------------------------------------
// PostUpdate run-once: re-pin AgX + upgrade sun shadows at High tier
// ---------------------------------------------------------------------------
//
// Runs after the first Update completes so it wins over bloom_pp's
// first-frame AcesFitted insertion.  The `run_once` condition from
// bevy::prelude ensures this fires exactly once per app launch.

fn pin_tonemapping_and_shadows(
    quality: Res<GraphicsQuality>,
    mut cameras: Query<&mut Tonemapping, With<Camera3d>>,
    mut sun_lights: Query<(Entity, &mut DirectionalLight)>,
    mut commands: Commands,
) {
    // Re-pin tonemapping: bloom_pp.rs inserts AcesFitted in its first Update
    // pass.  We want AgX for outdoor daylight — pin it here so our choice wins.
    for mut tm in &mut cameras {
        *tm = Tonemapping::AgX;
    }

    // Patch the sun DirectionalLight spawned by sky.rs (shadows_enabled: false
    // there for broad GPU compatibility).  At High tier we can afford shadows
    // with a multi-cascade configuration tuned for the 240-m view distance.
    // WebGL2 only supports a single shadow cascade; native uses 4.
    if *quality == GraphicsQuality::High {
        for (entity, mut light) in &mut sun_lights {
            if !light.shadows_enabled {
                light.shadows_enabled    = true;
                light.shadow_depth_bias  = DirectionalLight::DEFAULT_SHADOW_DEPTH_BIAS;
                light.shadow_normal_bias = DirectionalLight::DEFAULT_SHADOW_NORMAL_BIAS;

                // CascadeShadowConfigBuilder::default() already picks num_cascades=1
                // on wasm32/webgl and num_cascades=4 on native, so we rely on that
                // rather than hard-coding 4, which would break WebGL2.
                let cascade_config = CascadeShadowConfigBuilder {
                    maximum_distance: 240.0,
                    first_cascade_far_bound: 10.0,
                    overlap_proportion: 0.1,
                    ..default()
                }
                .build();
                commands.entity(entity).insert(cascade_config);

                info!("post_fx: enabled shadows on sun DirectionalLight (240 m)");
            }
        }
    }
}
