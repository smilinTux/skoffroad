# skoffroad — mobile smoke tests

Playwright-based integration tests that exercise the HTML overlay UI (touch
controls, mobile menu, joystick) and validate that the WASM game boots and
renders frames on a headless Chromium instance emulating an iPhone 14 viewport.

## Quick start — full local run

```bash
./tests/mobile/run-local.sh
```

The script is idempotent and handles everything end-to-end:

1. Installs `trunk` if missing (`cargo install trunk --locked`)
2. Adds the `wasm32-unknown-unknown` rustup target if missing
3. Builds the WASM release bundle (`trunk build --release` → `dist/`)
4. Serves `dist/` on `http://localhost:8080` via `npx http-server`
5. Installs the Playwright Chromium browser if missing
6. Runs the full Playwright test suite (`npx playwright test`)
7. Tears down the static server on exit and prints **PASS** or **FAIL**

**First run is slow** (5–15 min) because Rust compiles ~34 MB of WASM.
Subsequent runs use the cargo / trunk build cache and finish in under a minute.

## Fast native gate (no browser)

```bash
cargo test --test drive_test
```

Runs the pure-Rust physics simulation unit tests.  These have no WASM or
browser dependency and complete in seconds.

## Test architecture

The suite is split into two tiers:

| Tier | # tests | Depends on WASM? | Failure mode |
|------|---------|-----------------|--------------|
| HTML overlay (Tier 1) | 12 | No — DOM + JS only | Deterministic; should never flake |
| Canvas render (Tier 2) | 1 | Yes — needs GPU/WASM | May be slow on software-GL CI |

**Tier 1** tests interact with `#mobile-start`, `#tc-btn-*`, the joystick, and
the mobile menu overlay.  They use `tap({ force: true })` to bypass Playwright's
actionability/stability check (which the CSS pulse animation and WASM main-thread
jank would otherwise defeat) and wait only for elements to be **attached** in the
DOM, not visible/stable.

**Tier 2** polls the canvas for non-black pixels for up to 60 s with 2 retries.
It is marked `test.slow()` so Playwright triples its timeout budget.  If a
no-GPU CI runner consistently cannot render in time, mark this test
`test.fixme()` rather than letting it block the whole suite.

## Configuration

- `playwright.config.ts` — `retries: 2`, per-test `timeout: 90_000`, Chromium
  under iPhone 14 viewport
- Environment variable `PLAYWRIGHT_BASE_URL` overrides the default
  `http://localhost:8080` base URL

## Viewing the HTML report

```bash
cd tests/mobile && npx playwright show-report playwright-report
```
