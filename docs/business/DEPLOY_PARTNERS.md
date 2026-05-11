# Deploying the Partners Landing Page

The partners page lives at `web/partners/index.html` and ships to GitHub Pages
alongside the WASM build via `.github/workflows/release.yml`. The release
workflow now copies `web/partners/` into `dist/partners/` before upload.

After the next tagged release, the page is live at:

**https://play.skoffroad.skworld.io/partners/**

## One-time setup before first deploy

### 1. Create the Formspree form

The contact form currently posts to `https://formspree.io/f/REPLACE_ME`.

1. Sign up at https://formspree.io (free plan = 50 submissions/month).
2. Create a new form named "skoffroad partners".
3. Set the recipient email to **partnerships@skworld.io**.
4. Copy the form ID (the hex string after `/f/` in the action URL).
5. Replace `REPLACE_ME` in `web/partners/index.html` (one occurrence on the
   `<form action="...">` line) with your form ID.
6. Commit and push.

```bash
sed -i 's|https://formspree.io/f/REPLACE_ME|https://formspree.io/f/<YOUR_FORM_ID>|' \
    web/partners/index.html
```

### 2. (Optional) Apex domain → /partners/

If you want the partners page reachable at `skoffroad.skworld.io/partners/`
(no `play.` subdomain), the cleanest path is one of:

- **Cloudflare page rule** that rewrites `skoffroad.skworld.io/partners/*` →
  `play.skoffroad.skworld.io/partners/$1` and proxies. Zero downtime, zero
  extra GitHub setup. Recommended.
- **Separate GitHub Pages site** at `skoffroad.skworld.io` apex. Requires a
  second repo or a new gh-pages branch with its own `CNAME`. Use only if you
  want the apex to host a full marketing site, not just the partners page.

For the MVP, leave it at `play.skoffroad.skworld.io/partners/` and pitch that
URL in outreach. It's clean and works today.

## Deploy

Triggered by either:

```bash
# Tag a release
git tag v0.26.0 && git push --tags

# Or run from the GitHub Actions tab → "Release binaries" → Run workflow
```

The `wasm` job builds, the `pages-deploy` job publishes. ~6 minutes end-to-end.

## Verifying after deploy

```bash
# Should return 200 OK with text/html
curl -I https://play.skoffroad.skworld.io/partners/

# Spot-check the form action wired correctly
curl -s https://play.skoffroad.skworld.io/partners/ | grep formspree
```

## Updating the page without a full release

If you want to iterate on the partners page faster than the release cadence,
the cheapest option is the manual workflow_dispatch trigger:

1. Go to Actions → "Release binaries" → Run workflow
2. Pass the current latest tag as input
3. Same dist/ goes back up with the new `web/partners/` content

Or open a PR with the page changes, merge to `main`, then run
workflow_dispatch with the latest tag.

If this becomes painful, add a dedicated workflow that triggers on push to
`main` when `web/partners/**` changes. (Don't add it preemptively — it
conflicts with the existing `github-pages` environment unless you set up a
separate Pages site.)
