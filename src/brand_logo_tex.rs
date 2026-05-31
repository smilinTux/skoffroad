// brand_logo_tex.rs — Sprint 85
//
// Generates a procedural "logo plate" Image per parody brand at Startup.
// Each 256×128 RGBA texture encodes:
//   • Background filled with the brand primary color.
//   • A simple geometric logo-mark in the secondary color (shape chosen by
//     BrandCategory: Tires=mountain triangle, Shocks=chevron, Lights=lightning
//     bolt, Winches=circle-slash, Wheels=hex-ring, Recovery/Lifestyle=shield).
//   • The brand display_name in BLOCK LETTERS using a hand-coded 5x7 bitmap
//     font — no font asset required; each glyph is a [u8; 7] bitmask of 5 cols.
//
// Quality gate: texture generation is skipped on Low / headless; sign/livery
// systems fall back to the plain branded-color panel in that case.
//
// Public API:
//   BrandLogoTexPlugin
//   BrandLogoTextures  (Resource — Vec<Handle<Image>>, indexed by brand_id)
//   brand_logo_texture (helper — returns Option<Handle<Image>> for a brand_id)

use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

use crate::graphics_quality::GraphicsQuality;
use crate::parody_brands::{BrandCategory, ParodyBrands};

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct BrandLogoTexPlugin;

impl Plugin for BrandLogoTexPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, generate_brand_logos);
    }
}

// ---------------------------------------------------------------------------
// Resource
// ---------------------------------------------------------------------------

/// One Handle<Image> per brand, same indexing as ParodyBrands::brands.
/// Empty Vec on Low / headless (no GPU, no texture gen).
#[derive(Resource, Default)]
pub struct BrandLogoTextures {
    pub handles: Vec<Handle<Image>>,
}

/// Convenience accessor: returns the logo texture handle for `brand_id`, or
/// `None` if the textures were not generated (Low quality / headless).
pub fn brand_logo_texture(
    textures: &BrandLogoTextures,
    brand_id: usize,
) -> Option<Handle<Image>> {
    textures.handles.get(brand_id).cloned()
}

// ---------------------------------------------------------------------------
// Texture dimensions
// ---------------------------------------------------------------------------

/// Landscape plate: 256 wide x 128 tall. Matches sign panel aspect (2:1).
const TEX_W: usize = 256;
const TEX_H: usize = 128;

// ---------------------------------------------------------------------------
// Startup system
// ---------------------------------------------------------------------------

fn generate_brand_logos(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    brands: Res<ParodyBrands>,
    quality: Res<GraphicsQuality>,
) {
    // Low quality / headless: insert empty resource so callers get None handles.
    if *quality == GraphicsQuality::Low {
        commands.insert_resource(BrandLogoTextures::default());
        info!("brand_logo_tex: Low quality — skipping logo texture generation");
        return;
    }

    let mut handles: Vec<Handle<Image>> = Vec::with_capacity(brands.len());

    for brand in brands.iter() {
        let img = build_logo_plate(
            brand.primary,
            brand.secondary,
            brand.display_name,
            brand.category,
        );
        handles.push(images.add(img));
    }

    info!(
        "brand_logo_tex: generated {} logo-plate textures ({}x{})",
        handles.len(),
        TEX_W,
        TEX_H
    );
    commands.insert_resource(BrandLogoTextures { handles });
}

// ---------------------------------------------------------------------------
// Plate builder
// ---------------------------------------------------------------------------

fn build_logo_plate(
    primary: [f32; 3],
    secondary: [f32; 3],
    display_name: &str,
    category: BrandCategory,
) -> Image {
    let mut data: Vec<u8> = vec![0u8; TEX_W * TEX_H * 4];

    // Convert float colors to u8 RGBA.
    let bg = to_rgba8(primary[0], primary[1], primary[2], 1.0);
    let fg = to_rgba8(secondary[0], secondary[1], secondary[2], 1.0);

    // 1. Fill background.
    for pixel in data.chunks_exact_mut(4) {
        pixel.copy_from_slice(&bg);
    }

    // 2. Draw logo mark (left-side of plate, vertically centered).
    //    Mark occupies a 40x40 region at (10, 44).
    let mark_x = 10usize;
    let mark_y = 44usize;
    let mark_size = 40usize;
    draw_mark(&mut data, category, mark_x, mark_y, mark_size, &fg);

    // 3. Draw brand name in 5x7 bitmap font.
    //    Scale factor 2: each glyph renders as 10x14 pixels.
    //    Name area: x >= 60, leaving 8px margin on right.
    let scale = 2usize;
    let char_w = 5 * scale; // 10 px per char
    let char_h = 7 * scale; // 14 px per char
    let gap    = 2usize;    // gap between chars in pixels
    let name_bytes = display_name.as_bytes();

    let total_name_w = if name_bytes.is_empty() {
        0
    } else {
        name_bytes.len() * (char_w + gap) - gap
    };
    let name_area_x = 60usize;
    let name_area_w = TEX_W.saturating_sub(name_area_x + 8);
    let name_start_x = if total_name_w < name_area_w {
        name_area_x + (name_area_w - total_name_w) / 2
    } else {
        name_area_x
    };
    // Vertically centre the text in the plate.
    let name_start_y = (TEX_H.saturating_sub(char_h)) / 2;

    let mut cx = name_start_x;
    for &b in name_bytes {
        if cx + char_w > TEX_W { break; }
        let glyph = glyph_for(b);
        draw_glyph(&mut data, &glyph, cx, name_start_y, scale, &fg);
        cx += char_w + gap;
    }

    Image::new(
        Extent3d {
            width:               TEX_W as u32,
            height:              TEX_H as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

// ---------------------------------------------------------------------------
// Logo mark drawing
// ---------------------------------------------------------------------------

fn draw_mark(
    data: &mut [u8],
    category: BrandCategory,
    ox: usize,
    oy: usize,
    size: usize,
    color: &[u8; 4],
) {
    match category {
        // Mountain triangle — Tires
        BrandCategory::Tires => {
            // Filled upward-pointing triangle: apex at top-centre, base at bottom.
            for row in 0..size {
                let half_w = row * (size / 2) / size.max(1);
                let mid   = ox + size / 2;
                let left  = mid.saturating_sub(half_w);
                let right = (mid + half_w + 1).min(ox + size);
                for col in left..right {
                    set_pixel(data, col, oy + row, color);
                }
            }
        }
        // Chevron (V-shape) — Shocks
        BrandCategory::Shocks => {
            // Two thick diagonal arms meeting at the bottom-centre.
            let thickness = (size / 8).max(2);
            for i in 0..size {
                for t in 0..thickness {
                    // Left arm: from top-left corner down to bottom-centre.
                    let lx = ox + i / 2 + t;
                    let ly = oy + i;
                    if lx < ox + size {
                        set_pixel(data, lx, ly, color);
                    }
                    // Right arm: mirror.
                    if ox + size > 0 {
                        let rx_base = ox + size - 1 - i / 2;
                        let rx = rx_base.saturating_sub(t);
                        if rx >= ox {
                            set_pixel(data, rx, ly, color);
                        }
                    }
                }
            }
        }
        // Lightning bolt — Lights
        BrandCategory::Lights => {
            // Zigzag bolt: upper diagonal (top-right to mid-left) +
            // lower diagonal (mid-right to bottom-left).
            let half  = size / 2;
            let thick = (size / 7).max(2);
            // Upper half: x goes from (size-1) down to (size/2), y from 0 to half.
            for i in 0..half {
                for t in 0..thick {
                    let x = ox + size - 1 - i + t;
                    let y = oy + i;
                    if x < ox + size {
                        set_pixel(data, x.min(ox + size - 1), y, color);
                    }
                }
            }
            // Lower half: x goes from (size/2) down to 0, y from half to size.
            for i in 0..half {
                for t in 0..thick {
                    let x = ox + half - i + t;
                    let y = oy + half + i;
                    if x < ox + size {
                        set_pixel(data, x.min(ox + size - 1), y, color);
                    }
                }
            }
        }
        // Circle-slash — Winches
        BrandCategory::Winches => {
            let cx_i = (ox + size / 2) as i32;
            let cy_i = (oy + size / 2) as i32;
            let r    = (size / 2).saturating_sub(2) as i32;
            let ring = (size / 8).max(2) as i32;
            let inner = (r - ring).max(0);
            for row in 0..size {
                for col in 0..size {
                    let dx = (ox + col) as i32 - cx_i;
                    let dy = (oy + row) as i32 - cy_i;
                    let d2 = dx * dx + dy * dy;
                    // Ring: inner^2 <= d2 <= r^2.
                    let on_ring = d2 >= inner * inner && d2 <= r * r;
                    // Diagonal slash (dx + dy = 0 line), thickness = ring.
                    let on_slash = (dx + dy).abs() <= ring && d2 <= r * r;
                    if on_ring || on_slash {
                        set_pixel(data, ox + col, oy + row, color);
                    }
                }
            }
        }
        // Hex ring — Wheels
        BrandCategory::Wheels => {
            let cx_i = (ox + size / 2) as i32;
            let cy_i = (oy + size / 2) as i32;
            let r    = (size / 2).saturating_sub(3) as i32;
            let ring = (size / 8).max(2) as i32;
            for row in 0..size {
                for col in 0..size {
                    let dx = (ox + col) as i32 - cx_i;
                    let dy = (oy + row) as i32 - cy_i;
                    if in_hex(dx, dy, r) && !in_hex(dx, dy, (r - ring).max(0)) {
                        set_pixel(data, ox + col, oy + row, color);
                    }
                }
            }
        }
        // Shield — Recovery / Lifestyle / default
        BrandCategory::Recovery | BrandCategory::Lifestyle => {
            // Pentagon/shield: triangle top + rectangle body tapering to a point.
            let top_h = size / 3;
            let half_w = size / 2;
            // Triangle top: grows from 0 width at apex to half_w at body_top.
            for row in 0..top_h {
                let w = (row + 1) * half_w / top_h.max(1);
                let mid = ox + half_w;
                for col in mid.saturating_sub(w)..(mid + w).min(ox + size) {
                    set_pixel(data, col, oy + row, color);
                }
            }
            // Rectangle body: full width until bottom-quarter where it tapers.
            for row in top_h..size {
                let frac = (size - row) as i32;
                let w = if row >= size * 3 / 4 {
                    // Taper to point.
                    ((frac * half_w as i32) / (size / 4).max(1) as i32).max(1) as usize
                } else {
                    half_w
                };
                let mid = ox + half_w;
                for col in mid.saturating_sub(w)..(mid + w).min(ox + size) {
                    set_pixel(data, col, oy + row, color);
                }
            }
        }
    }
}

/// Regular hexagon test via the three-axis approach.
fn in_hex(dx: i32, dy: i32, r: i32) -> bool {
    if r <= 0 { return false; }
    let adx = dx.unsigned_abs() as i32;
    let ady = dy.unsigned_abs() as i32;
    // For a flat-top hex of radius r: |dy| <= r AND |dx|*866/1000 + |dy|/2 <= r.
    ady <= r && adx * 866 / 1000 + ady / 2 <= r
}

// ---------------------------------------------------------------------------
// 5x7 bitmap font — A-Z, 0-9, space, punctuation
// ---------------------------------------------------------------------------
//
// Each glyph is [u8; 7] — one byte per row (7 rows), 5 bits per row.
// Bit 4 = column 0 (leftmost), bit 0 = column 4 (rightmost).
// 1 = draw foreground pixel.

type Glyph = [u8; 7];

const GLYPH_SPACE: Glyph = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

#[rustfmt::skip]
const GLYPHS_AZ: [Glyph; 26] = [
    // A              B              C              D              E
    [0x04,0x0A,0x11,0x1F,0x11,0x11,0x11],
    // B
    [0x1E,0x11,0x11,0x1E,0x11,0x11,0x1E],
    // C
    [0x0E,0x11,0x10,0x10,0x10,0x11,0x0E],
    // D
    [0x1E,0x09,0x11,0x11,0x11,0x09,0x1E],
    // E
    [0x1F,0x10,0x10,0x1E,0x10,0x10,0x1F],
    // F
    [0x1F,0x10,0x10,0x1E,0x10,0x10,0x10],
    // G
    [0x0E,0x11,0x10,0x17,0x11,0x11,0x0E],
    // H
    [0x11,0x11,0x11,0x1F,0x11,0x11,0x11],
    // I
    [0x0E,0x04,0x04,0x04,0x04,0x04,0x0E],
    // J
    [0x07,0x02,0x02,0x02,0x02,0x12,0x0C],
    // K
    [0x11,0x12,0x14,0x18,0x14,0x12,0x11],
    // L
    [0x10,0x10,0x10,0x10,0x10,0x10,0x1F],
    // M
    [0x11,0x1B,0x15,0x11,0x11,0x11,0x11],
    // N
    [0x11,0x19,0x15,0x13,0x11,0x11,0x11],
    // O
    [0x0E,0x11,0x11,0x11,0x11,0x11,0x0E],
    // P
    [0x1E,0x11,0x11,0x1E,0x10,0x10,0x10],
    // Q
    [0x0E,0x11,0x11,0x11,0x15,0x12,0x0D],
    // R
    [0x1E,0x11,0x11,0x1E,0x14,0x12,0x11],
    // S
    [0x0E,0x11,0x10,0x0E,0x01,0x11,0x0E],
    // T
    [0x1F,0x04,0x04,0x04,0x04,0x04,0x04],
    // U
    [0x11,0x11,0x11,0x11,0x11,0x11,0x0E],
    // V
    [0x11,0x11,0x11,0x11,0x11,0x0A,0x04],
    // W
    [0x11,0x11,0x11,0x15,0x15,0x1B,0x11],
    // X
    [0x11,0x11,0x0A,0x04,0x0A,0x11,0x11],
    // Y
    [0x11,0x11,0x0A,0x04,0x04,0x04,0x04],
    // Z
    [0x1F,0x01,0x02,0x04,0x08,0x10,0x1F],
];

#[rustfmt::skip]
const GLYPHS_09: [Glyph; 10] = [
    // 0
    [0x0E,0x11,0x13,0x15,0x19,0x11,0x0E],
    // 1
    [0x04,0x0C,0x04,0x04,0x04,0x04,0x0E],
    // 2
    [0x0E,0x11,0x01,0x06,0x08,0x10,0x1F],
    // 3
    [0x1F,0x02,0x04,0x02,0x01,0x11,0x0E],
    // 4
    [0x02,0x06,0x0A,0x12,0x1F,0x02,0x02],
    // 5
    [0x1F,0x10,0x1E,0x01,0x01,0x11,0x0E],
    // 6
    [0x06,0x08,0x10,0x1E,0x11,0x11,0x0E],
    // 7
    [0x1F,0x01,0x02,0x04,0x08,0x08,0x08],
    // 8
    [0x0E,0x11,0x11,0x0E,0x11,0x11,0x0E],
    // 9
    [0x0E,0x11,0x11,0x0F,0x01,0x02,0x0C],
];

fn glyph_for(b: u8) -> Glyph {
    match b {
        b'A'..=b'Z' => GLYPHS_AZ[(b - b'A') as usize],
        b'a'..=b'z' => GLYPHS_AZ[(b - b'a') as usize],
        b'0'..=b'9' => GLYPHS_09[(b - b'0') as usize],
        b' '        => GLYPH_SPACE,
        b'-' | b'_' => [0x00,0x00,0x00,0x1F,0x00,0x00,0x00],
        b'&'        => [0x0C,0x12,0x14,0x08,0x15,0x12,0x0D],
        b'.'        => [0x00,0x00,0x00,0x00,0x00,0x0C,0x0C],
        b'!'        => [0x04,0x04,0x04,0x04,0x04,0x00,0x04],
        b'/'        => [0x01,0x02,0x02,0x04,0x08,0x08,0x10],
        _           => [0x1F,0x11,0x11,0x11,0x11,0x11,0x1F],
    }
}

/// Blit a glyph at (ox, oy) scaled by `scale`.
/// scale=2 -> each dot becomes a 2x2 block -> 10x14 px per character.
fn draw_glyph(
    data: &mut [u8],
    glyph: &Glyph,
    ox: usize,
    oy: usize,
    scale: usize,
    color: &[u8; 4],
) {
    for (row, &byte) in glyph.iter().enumerate() {
        for col in 0..5usize {
            let bit = (byte >> (4 - col)) & 1;
            if bit == 0 { continue; }
            for dy in 0..scale {
                for dx in 0..scale {
                    set_pixel(data, ox + col * scale + dx, oy + row * scale + dy, color);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Pixel helpers
// ---------------------------------------------------------------------------

#[inline]
fn set_pixel(data: &mut [u8], x: usize, y: usize, color: &[u8; 4]) {
    if x >= TEX_W || y >= TEX_H { return; }
    let idx = (y * TEX_W + x) * 4;
    data[idx]     = color[0];
    data[idx + 1] = color[1];
    data[idx + 2] = color[2];
    data[idx + 3] = color[3];
}

#[inline]
fn to_rgba8(r: f32, g: f32, b: f32, a: f32) -> [u8; 4] {
    [
        (r.clamp(0.0, 1.0) * 255.0) as u8,
        (g.clamp(0.0, 1.0) * 255.0) as u8,
        (b.clamp(0.0, 1.0) * 255.0) as u8,
        (a.clamp(0.0, 1.0) * 255.0) as u8,
    ]
}
