# Brand Packs

Brand packs let a single skoffroad build present a different brand's livery, billboards, and splash screen without recompiling.

## Quick start (for our internal use)

1. Duplicate `example_brand.json` → `assets/brand_packs/<client_id>.json`.
2. Drop brand textures in `assets/brand_packs/<client_id>/` (logo, decals, billboards).
3. Run the game with `--brand <client_id>` flag, or set `default_brand_pack` in `~/.skoffroad/config.json`.
4. The active pack swaps:
   - Splash screen logo + tagline + CTA
   - Selectable livery (one entry added to the livery cycler)
   - Roadside billboards (replaces house ads with brand creatives)

The default pack `_house.json` ships with the game and presents "Want your brand here?" billboards that link to the partners landing page.

## Schema

See [`example_brand.json`](./example_brand.json) for a fully-commented template.

Required fields: `id`, `display_name`, `version`, `splash.cta_url`, at least one billboard.

Optional fields: `livery` (omit if no custom livery), individual `texture` fields (omit + use `fallback_color` for solid-color placeholder billboards).

## Authoring guidelines

| Asset | Format | Resolution |
| --- | --- | --- |
| Billboard texture | PNG, sRGB | 2048×1024 (16:8) |
| Logo | PNG with alpha | 1024×256 |
| Door decal | PNG with alpha | 1024×1024 |

Keep total brand-pack asset size under **8 MB gzipped** — the game ships ~12 MB and we don't want to double that for one brand.

## UTM tracking

Every `click_url` and `cta_url` should include UTM parameters so the brand's analytics can attribute traffic. Recommended params:

- `utm_source=skoffroad`
- `utm_medium=billboard` (or `livery`, `splash`)
- `utm_campaign=<campaign_id>`
- `utm_content=<creative_id>` (lets the brand A/B different boards)

## Validation

Run `cargo run --bin validate_brand_pack -- assets/brand_packs/<id>.json` (TODO) to verify:

- All referenced textures exist on disk
- CTA / click URLs are well-formed HTTPS URLs
- Color values are in 0.0–1.0 range
- No required field is missing

## Privacy

- No PII is collected by the game from brand packs.
- Brand packs cannot inject arbitrary code — only declarative data + texture/audio assets.
- UTM-tagged click-throughs are the only outbound signal.
