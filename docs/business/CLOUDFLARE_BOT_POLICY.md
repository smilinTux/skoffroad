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

The "Block AI Scrapers and Crawlers" toggle in the dashboard is a managed WAF
rule, which the current API token can't edit (it has `Zone Settings:Edit`,
not `Zone WAF:Edit`).

**Per zone, do this once:**

1. Cloudflare dashboard → pick zone → **Security** → **Bots**
2. Find **"Block AI Scrapers and Crawlers"**
3. Toggle it **OFF**.
4. Save.

This takes ~10 seconds per zone, 26 zones ≈ 5 minutes. Or:

**To do it via API**, create a new token with these scopes:

- Zone WAF: Edit
- Zone Settings: Read

Then run the script in [`scripts/cf_disable_ai_bot_block.sh`](../../scripts/cf_disable_ai_bot_block.sh)
(future — not written yet because the existing token lacks the scope).

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

### Option B — Cloudflare Worker, one script for all zones

A single Worker that intercepts `/robots.txt` on every zone and returns the
permissive policy. Setup:

1. Create a Worker named `permissive-robots-txt`.
2. Source:
   ```javascript
   const ROBOTS = `User-agent: *\nAllow: /\n`;
   // (paste the full file from skoffroad/robots.txt)
   export default {
     fetch(req) {
       const url = new URL(req.url);
       if (url.pathname === '/robots.txt') {
         return new Response(ROBOTS, {
           headers: { 'content-type': 'text/plain; charset=utf-8' },
         });
       }
       return fetch(req); // pass through everything else
     }
   };
   ```
3. Add a Worker route on each of the 26 zones: `*/robots.txt`.

This is more upfront work but uniform across all 26 domains and free up to
100k requests/day per account.

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
