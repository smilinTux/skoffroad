#!/usr/bin/env bash
# Bulk-attach */robots.txt routes for the permissive-robots-txt Worker on
# every zone the CF_TOKEN can see.
#
# Usage:
#   export CF_TOKEN='<token with Zone:Edit + Zone:Read on all zones>'
#   bash scripts/attach_robots_routes.sh

set -euo pipefail

: "${CF_TOKEN:?Set CF_TOKEN to a Cloudflare API token with Zone:Edit}"
WORKER_NAME="${WORKER_NAME:-permissive-robots-txt}"

API="https://api.cloudflare.com/client/v4"

echo "Discovering zones..."
ZONES_JSON=$(curl -fsS -X GET "$API/zones?per_page=50&status=active" \
  -H "Authorization: Bearer $CF_TOKEN")

# Extract id\tname pairs.
echo "$ZONES_JSON" | python3 -c "
import sys, json
d = json.load(sys.stdin)
if not d['success']:
    print('ERROR:', d['errors'], file=sys.stderr); sys.exit(1)
for z in d['result']:
    print(f\"{z['id']}\t{z['name']}\")
" | while IFS=$'\t' read -r ZID ZNAME; do
    PATTERN="${ZNAME}/robots.txt"
    echo -n "  ${ZNAME}: "
    RESULT=$(curl -fsS -X POST "$API/zones/$ZID/workers/routes" \
        -H "Authorization: Bearer $CF_TOKEN" \
        -H "Content-Type: application/json" \
        -d "{\"pattern\":\"$PATTERN\",\"script\":\"$WORKER_NAME\"}" 2>&1 || true)
    if echo "$RESULT" | grep -q '"success":true'; then
        echo "✓ attached"
    elif echo "$RESULT" | grep -q '"code":10020'; then
        # Route already exists. Re-PUT to update.
        echo "(exists, leaving as-is)"
    else
        echo "FAIL: $RESULT"
    fi
done

echo
echo "Done. Test with:"
echo "  curl -i https://skworld.io/robots.txt"
