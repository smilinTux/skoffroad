# permissive-robots-txt Worker

One Cloudflare Worker that serves a permissive `/robots.txt` on every zone
it's routed to. Everything else passes through to the origin unchanged.

This gives all 26 skworld Cloudflare zones the same crawler policy with one
deploy + a batch route attachment.

## Why a Worker

- The other 25 zones don't share a Git repo or origin — each is hosted
  differently or not yet hosted. A Worker intercepts at the Cloudflare edge,
  so the policy is uniform regardless of what's behind each origin.
- Workers Free tier: 100,000 requests/day per account, plenty for `/robots.txt`
  traffic on 26 zones.
- One file to update; redeploys propagate to all 26 zones instantly.

## Prerequisites

```bash
# Install wrangler if you don't have it
npm install -g wrangler

# Authenticate (opens a browser tab)
wrangler login
```

## Deploy

```bash
cd scripts/cf-robots-worker
wrangler deploy
```

This uploads the Worker script (no routes attached yet). Output will include
the Worker's URL like `permissive-robots-txt.<your-subdomain>.workers.dev`.

## Attach routes to all 26 zones

After the Worker exists, attach `*/robots.txt` routes on each zone. The
attach script needs a token with `Zone:Edit` (the existing token already
has Zone Settings:Edit, which includes routes).

```bash
export CF_TOKEN='<your_token>'
bash scripts/attach_robots_routes.sh
```

The script enumerates all active zones and creates one route per zone
pointing at this Worker.

## Test

```bash
# Pick any of your zones and curl /robots.txt
curl -i https://skarchitect.io/robots.txt
# Should return the permissive policy with x-served-by: cf-permissive-robots-worker
```

## Updating the policy later

Edit `src/worker.js` and rerun `wrangler deploy`. Routes stay attached; only
the response body updates. ~5 seconds end-to-end.

## Removing

```bash
# Detach all routes
bash scripts/detach_robots_routes.sh  # (write if needed)

# Delete the Worker
wrangler delete permissive-robots-txt
```
