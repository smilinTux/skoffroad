# Cloudflare Bot / Crawler Policy

Goal: **maximum-permissive crawler access** across all 26 skworld zones. We
want every AI / search / answer-engine to crawl freely so the game and the
partners page surface in AI answers, search results, and link previews.

## What was applied (via API, 2026-05-11)

For all 26 zones the token can reach, two zone settings were patched:

| Setting | Before | After | Effect |
| --- | --- | --- | --- |
| `security_level` | `medium` | `essentially_off` | Almost no blocking based on threat score |
| `browser_check` | `on` | `off` | No JS-challenge for crawlers that don't render |

```bash
# Per-zone, this is what each PATCH was:
curl -X PATCH "https://api.cloudflare.com/client/v4/zones/$ZONE/settings/security_level" \
     -H "Authorization: Bearer $CF_TOKEN" \
     -H "Content-Type: application/json" \
     -d '{"value":"essentially_off"}'

curl -X PATCH "https://api.cloudflare.com/client/v4/zones/$ZONE/settings/browser_check" \
     -H "Authorization: Bearer $CF_TOKEN" \
     -H "Content-Type: application/json" \
     -d '{"value":"off"}'
```

## Zones covered

```
ailife.family                       skarchitect.io      skstacks.io
beautifulsegfault.com               skcapstone.io       skvector.io
bingchilling.org                    skchat.io           skworld.io
capauth.io                          skcomm.io           smilintux.org
douno.it                            skdata.io           staycuriousandkeepsmil.in
feralholding.com                    skforge.io          staycuriousandkeepsmiling.com
forgeprint.io                       skgraph.io          staycuriouskeepsmilin.com
skaid.io                            skhelp.io           staycuriouskeepsmiling.com
                                    skmemory.io
                                    sksecurity.io
```

## What still needs the dashboard (one toggle per zone)

The "Block AI Scrapers and Crawlers" toggle in the dashboard is **not exposed
via Cloudflare's public API on Free plans**. Even with `Zone WAF:Edit` on the
token, the relevant endpoints (`/zones/{id}/bot_management`,
`/zones/{id}/rulesets/phases/http_request_dynamic_redirect/entrypoint`) all
return `request is not authorized` or `Authentication error` — Cloudflare
gates them behind Bot Management (Pro+) or behind scopes it doesn't grant
to standalone tokens on Free plans.

**Per zone, do this once:**

1. Cloudflare dashboard → pick zone → **Security** → **Bots**
2. Find **"Block AI Scrapers and Crawlers"**
3. Toggle it **OFF**.
4. Save.

This takes ~10 seconds per zone, 26 zones ≈ 5 minutes. Or:

Verified empirically — there's no API path that works for the Free-plan AI-bot
toggle as of 2026-05-11. If/when Cloudflare exposes it, revisit. For now,
**the 26 dashboard toggles are the only path**.

## robots.txt

A maximum-permissive `robots.txt` was added at the skoffroad repo root and is
served via Trunk at `play.skoffroad.skworld.io/robots.txt`. It explicitly
welcomes every major AI crawler (GPTBot, ClaudeBot, PerplexityBot,
Google-Extended, CCBot, Bytespider, etc.) and every social unfurler
(Twitterbot, facebookexternalhit, LinkedInBot, Slackbot, Discordbot).

**For the other 25 zones**, each needs its own `robots.txt` at the origin's
web root. Two paths to do this uniformly across all of them:

### Option A — Per-origin (recommended if origins differ)

Add a `robots.txt` to whatever serves each domain. The file in
`skoffroad/robots.txt` is a good template — just copy and update the
`Sitemap:` line if there is one.

### Option B — Cloudflare Worker, one script for all zones (Recommended)

A single Worker that intercepts `/robots.txt` on every zone and returns the
permissive policy. **The full Worker, wrangler config, and route-attach
script are committed under [`scripts/cf-robots-worker/`](../../scripts/cf-robots-worker/)
and [`scripts/attach_robots_routes.sh`](../../scripts/attach_robots_routes.sh).**

To deploy (takes ~2 min total):

```bash
# 1. Install wrangler if needed
npm install -g wrangler

# 2. Authenticate (opens browser)
wrangler login

# 3. Deploy the Worker
cd scripts/cf-robots-worker
wrangler deploy

# 4. Attach */robots.txt routes to all 26 zones via API
export CF_TOKEN='<your_token>'
bash ../attach_robots_routes.sh

# 5. Verify on any of your domains
curl -i https://skworld.io/robots.txt
```

The Worker pass-throughs everything except `/robots.txt`, so origins are
unaffected. Uses Cloudflare Workers Free tier (100k req/day per account —
plenty for `/robots.txt` traffic on 26 zones).

## Reverting

If we ever need to lock things back down:

```bash
# Per zone:
curl -X PATCH "https://api.cloudflare.com/client/v4/zones/$ZONE/settings/security_level" \
     -H "Authorization: Bearer $CF_TOKEN" \
     -d '{"value":"medium"}'

curl -X PATCH "https://api.cloudflare.com/client/v4/zones/$ZONE/settings/browser_check" \
     -H "Authorization: Bearer $CF_TOKEN" \
     -d '{"value":"on"}'
```

## Security note

The API tokens used for this work were pasted into the chat transcript
during the session. **Rotate them in Cloudflare** before next use:
https://dash.cloudflare.com/profile/api-tokens
