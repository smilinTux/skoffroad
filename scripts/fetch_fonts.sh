#!/usr/bin/env bash
# Fetch OFL fonts from GitHub releases.
#
# Idempotent: if every font is already present, the asset is skipped.
# Re-run with --force to redownload.
#
# Fonts installed:
#   assets/fonts/primary.ttf  ← JetBrains Mono Regular (OFL-1.1)
#   assets/fonts/display.ttf  ← Inter Display Regular  (OFL-1.1)

set -euo pipefail

cd "$(dirname "$0")/.."

FORCE=0
[[ "${1:-}" == "--force" ]] && FORCE=1

OUT_DIR="assets/fonts"
mkdir -p "$OUT_DIR"

TMP=$(mktemp -d)
trap "rm -rf '$TMP'" EXIT

# ── JetBrains Mono Regular → primary.ttf ────────────────────────────────────
JBMONO_VER="v2.304"
JBMONO_ZIP="JetBrainsMono-2.304.zip"
JBMONO_URL="https://github.com/JetBrains/JetBrainsMono/releases/download/${JBMONO_VER}/${JBMONO_ZIP}"
JBMONO_TTF="fonts/ttf/JetBrainsMono-Regular.ttf"

if [[ $FORCE -eq 0 && -f "$OUT_DIR/primary.ttf" ]]; then
  echo "[skip] primary.ttf (JetBrains Mono) already present"
else
  echo "[get ] JetBrains Mono ${JBMONO_VER} <- $JBMONO_URL"
  curl -sSL --max-time 120 -L "$JBMONO_URL" -o "$TMP/$JBMONO_ZIP"
  unzip -q -o "$TMP/$JBMONO_ZIP" "$JBMONO_TTF" -d "$TMP/jbmono"
  cp "$TMP/jbmono/$JBMONO_TTF" "$OUT_DIR/primary.ttf"
  echo "[ok  ] primary.ttf -> $OUT_DIR/primary.ttf"
fi

# ── Inter Display Regular → display.ttf ─────────────────────────────────────
INTER_VER="v4.0"
INTER_ZIP="Inter-4.0.zip"
INTER_URL="https://github.com/rsms/inter/releases/download/${INTER_VER}/${INTER_ZIP}"
INTER_TTF="extras/ttf/InterDisplay-Regular.ttf"

if [[ $FORCE -eq 0 && -f "$OUT_DIR/display.ttf" ]]; then
  echo "[skip] display.ttf (Inter Display) already present"
else
  echo "[get ] Inter Display ${INTER_VER} <- $INTER_URL"
  curl -sSL --max-time 120 -L "$INTER_URL" -o "$TMP/$INTER_ZIP"
  unzip -q -o "$TMP/$INTER_ZIP" "$INTER_TTF" -d "$TMP/inter"
  cp "$TMP/inter/$INTER_TTF" "$OUT_DIR/display.ttf"
  echo "[ok  ] display.ttf -> $OUT_DIR/display.ttf"
fi

echo
echo "All fonts installed under $OUT_DIR/."
echo "Attribution: see $OUT_DIR/FONTS.md"
