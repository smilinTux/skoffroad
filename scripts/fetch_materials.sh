#!/usr/bin/env bash
# Fetch CC0 PBR terrain materials from ambientCG.
#
# Idempotent: if every map for an asset is already present, the asset is
# skipped. Re-run with --force to redownload and re-extract.
#
# Layout produced (all flattened, JPGs only):
#   assets/materials/terrain/<slot>/{albedo,normal,roughness}.jpg

set -euo pipefail

cd "$(dirname "$0")/.."

FORCE=0
[[ "${1:-}" == "--force" ]] && FORCE=1

# slot   asset_id         resolution
ASSETS=(
  "dirt:Ground054:1K"
  "grass:Grass001:1K"
  "rock:Rock047:1K"
  "mud:Ground068:1K"
)

TMP=$(mktemp -d)
trap "rm -rf '$TMP'" EXIT

for entry in "${ASSETS[@]}"; do
  slot="${entry%%:*}"
  rest="${entry#*:}"
  asset_id="${rest%%:*}"
  res="${rest##*:}"

  out_dir="assets/materials/terrain/$slot"
  if [[ $FORCE -eq 0 \
     && -f "$out_dir/albedo.jpg" \
     && -f "$out_dir/normal.jpg" \
     && -f "$out_dir/roughness.jpg" ]]; then
    echo "[skip] $slot ($asset_id) already present"
    continue
  fi

  zip_name="${asset_id}_${res}-JPG.zip"
  zip_path="$TMP/$zip_name"
  url="https://ambientcg.com/get?file=$zip_name"

  echo "[get ] $slot ($asset_id $res) <- $url"
  curl -sSL --max-time 120 -o "$zip_path" "$url"

  extract_dir="$TMP/$asset_id"
  mkdir -p "$extract_dir"
  unzip -q -o "$zip_path" -d "$extract_dir"

  mkdir -p "$out_dir"

  # ambientCG names files like Ground054_1K-JPG_Color.jpg.
  # Pick Color, NormalGL, Roughness; rename to albedo/normal/roughness.
  cp "$extract_dir/${asset_id}_${res}-JPG_Color.jpg"     "$out_dir/albedo.jpg"
  cp "$extract_dir/${asset_id}_${res}-JPG_NormalGL.jpg"  "$out_dir/normal.jpg"
  cp "$extract_dir/${asset_id}_${res}-JPG_Roughness.jpg" "$out_dir/roughness.jpg"

  echo "[ok  ] $slot -> $out_dir/{albedo,normal,roughness}.jpg"
done

echo
echo "All materials installed under assets/materials/terrain/."
echo "Attribution: see assets/materials/MATERIALS.md"
