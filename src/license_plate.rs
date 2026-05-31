// License plate — Sprint 86 vehicle detail pass.
//
// Bakes the text "SK-86" (a short plate tag) into a procedural RGBA texture
// using a hand-coded 5x7 ASCII bitmap-font glyph table (each glyph stored as
// 5 column bitmasks, 1 bit per pixel row).  The texture is applied via
// `base_color_texture` on a StandardMaterial — the robust, world-space-safe
// technique (no Unicode in Text::new, no tofu risk).
//
// Respawn-safety: attach_plate queries
//   Query<Entity, (With<Chassis>, Without<LicensePlateAttached>)>
// so a fresh chassis (post-RespawnRequest) gets the plate re-attached without
// relying on a Local<bool> that would permanently skip re-attachment.
//
// Public API:
//   LicensePlatePlugin

use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use crate::vehicle::{Chassis, VehicleRoot};

// ---- Constants ---------------------------------------------------------------

const PLATE_W: f32 = 0.60;
const PLATE_H: f32 = 0.25;
const PLATE_D: f32 = 0.04;

/// Plate position in chassis local space (just beyond rear face).
const PLATE_LOCAL: Vec3 = Vec3::new(0.0, -0.1, 2.02);

// ---- Texture dimensions ------------------------------------------------------

/// Plate texture: 128 wide x 48 tall pixels.  Large enough to read text at
/// close range without blurriness, small enough to allocate cheaply.
const TEX_W: usize = 128;
const TEX_H: usize = 48;

// ---- 5x7 bitmap glyph table --------------------------------------------------
//
// Each glyph is 5 columns wide.  Each column is a u8 bitmask:
//   bit 6 = top pixel row, bit 0 = bottom pixel row (7 rows used, bit 7 = 0).
//
// Glyphs needed: S K - 8 6

type Glyph = [u8; 5];

// S: top bar, left side, mid bar, right side, bottom bar
const GLYPH_S: Glyph = [
    0b0111110, // col 0
    0b1000001, // col 1
    0b1001001, // col 2
    0b1001001, // col 3
    0b0110010, // col 4
];

// K: full left bar plus two diagonals radiating outward
const GLYPH_K: Glyph = [
    0b1111111, // col 0 full left bar
    0b0001000, // col 1 mid point
    0b0010100, // col 2 inner diag
    0b0100010, // col 3 outer diag
    0b1000001, // col 4 corner tips
];

// - (dash): only middle 4 rows lit in first 4 cols
const GLYPH_DASH: Glyph = [
    0b0001000, // col 0
    0b0001000, // col 1
    0b0001000, // col 2
    0b0001000, // col 3
    0b0000000, // col 4
];

// 8: two rectangular loops sharing a mid-bar
const GLYPH_8: Glyph = [
    0b0111110, // col 0
    0b1010101, // col 1
    0b1010101, // col 2
    0b1010101, // col 3
    0b0111110, // col 4
];

// 6: left arc + closed bottom loop
const GLYPH_6: Glyph = [
    0b0111110, // col 0
    0b1001001, // col 1
    0b1001001, // col 2
    0b1001001, // col 3
    0b0000110, // col 4
];

// ---- Pixel colors (RGBA) -----------------------------------------------------

const BG:     [u8; 4] = [240, 238, 220, 255]; // warm white plate background
const INK:    [u8; 4] = [ 20,  20,  20, 255]; // near-black text
const BORDER: [u8; 4] = [180,  30,  30, 255]; // red accent border

// ---- Plugin ------------------------------------------------------------------

pub struct LicensePlatePlugin;

impl Plugin for LicensePlatePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, attach_plate);
    }
}

// ---- Marker components -------------------------------------------------------

#[derive(Component)]
struct LicensePlate;

/// Placed on the Chassis entity once the plate has been attached.
/// A respawned chassis won't carry this marker, so attach_plate will
/// re-fire and re-attach the plate — respawn-safe without Local<bool>.
#[derive(Component)]
struct LicensePlateAttached;

// ---- Attach system -----------------------------------------------------------

/// Runs every Update.  For each Chassis that does NOT yet have
/// LicensePlateAttached, waits for VehicleRoot then bakes and attaches the
/// plate.
fn attach_plate(
    vehicle:   Option<Res<VehicleRoot>>,
    chassis_q: Query<Entity, (With<Chassis>, Without<LicensePlateAttached>)>,
    mut commands:  Commands,
    mut meshes:    ResMut<Assets<Mesh>>,
    mut images:    ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(vehicle) = vehicle else { return };
    let Ok(chassis) = chassis_q.get(vehicle.chassis) else { return };

    // Bake the plate texture.
    let tex        = build_plate_texture();
    let tex_handle = images.add(tex);

    let plate_mesh = meshes.add(Cuboid::new(PLATE_W, PLATE_H, PLATE_D));
    let plate_mat  = materials.add(StandardMaterial {
        base_color_texture:   Some(tex_handle),
        base_color:           Color::WHITE, // white multiplied with texture = neutral
        perceptual_roughness: 0.55,
        metallic:             0.0,
        ..default()
    });

    let plate_entity = commands.spawn((
        LicensePlate,
        Mesh3d(plate_mesh),
        MeshMaterial3d(plate_mat),
        Transform::from_translation(PLATE_LOCAL),
    )).id();

    commands.entity(chassis).add_child(plate_entity);
    commands.entity(chassis).insert(LicensePlateAttached);
}

// ---- Texture builder ---------------------------------------------------------

/// Bakes "SK-86" onto a 128x48 sRGB RGBA image.
///
/// Layout:
///   - 2-pixel red border all around.
///   - Warm-white background inside the border.
///   - Five 5x7 glyphs centred vertically and horizontally, 2 px kerning.
fn build_plate_texture() -> Image {
    let mut pixels = vec![BG; TEX_W * TEX_H];

    // Red border (2 px thick).
    for y in 0..TEX_H {
        for x in 0..TEX_W {
            if x < 2 || x >= TEX_W - 2 || y < 2 || y >= TEX_H - 2 {
                pixels[y * TEX_W + x] = BORDER;
            }
        }
    }

    // Glyph sequence for "SK-86".
    let glyphs: &[Glyph] = &[GLYPH_S, GLYPH_K, GLYPH_DASH, GLYPH_8, GLYPH_6];
    let glyph_w: usize = 5;
    let glyph_h: usize = 7;
    let kern:    usize = 2; // pixels between glyphs

    // Total text pixel width.
    let text_w = glyphs.len() * glyph_w + (glyphs.len() - 1) * kern;

    // Horizontally centre within the 4-px inset zone.
    let inner_w = TEX_W.saturating_sub(8);
    let start_x = 4 + inner_w.saturating_sub(text_w) / 2;

    // Vertically centre within the 4-px inset zone.
    let inner_h = TEX_H.saturating_sub(8);
    let start_y = 4 + inner_h.saturating_sub(glyph_h) / 2;

    for (gi, glyph) in glyphs.iter().enumerate() {
        let gx = start_x + gi * (glyph_w + kern);
        for col in 0..glyph_w {
            let col_bits = glyph[col];
            for row in 0..glyph_h {
                // bit 6 = top row (row 0), bit 0 = bottom row (row 6)
                let bit = (col_bits >> (6 - row)) & 1;
                if bit == 1 {
                    let px = gx + col;
                    let py = start_y + row;
                    if px < TEX_W && py < TEX_H {
                        pixels[py * TEX_W + px] = INK;
                    }
                }
            }
        }
    }

    // Pack pixel array into raw bytes.
    let mut data: Vec<u8> = Vec::with_capacity(TEX_W * TEX_H * 4);
    for p in pixels {
        data.extend_from_slice(&p);
    }

    Image::new(
        Extent3d {
            width:                 TEX_W as u32,
            height:                TEX_H as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb, // sRGB -- correct for base_color_texture
        RenderAssetUsages::RENDER_WORLD,
    )
}
