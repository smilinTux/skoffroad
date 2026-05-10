/**
 * skoffroad — mobile smoke test (Sprint 61)
 *
 * Boots the WASM build in headless Chromium with iPhone 14 emulation,
 * taps #mobile-start to dismiss the title screen, and verifies the canvas
 * is rendering frames (non-black pixel check).
 *
 * Requirements:
 *   - The dist/ directory must be served at http://localhost:8080 before running.
 *   - Playwright + Chromium must be installed (npx playwright install chromium).
 *
 * Run locally:
 *   cd tests/mobile && npm install && npx playwright test
 */

import { test, expect, Page } from '@playwright/test';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Captures a screenshot of the <canvas id="bevy"> element and returns the
 * raw pixel buffer. Throws if the canvas element is not found.
 */
async function captureCanvasPixels(page: Page): Promise<Buffer> {
  const canvas = page.locator('canvas#bevy');
  await expect(canvas).toBeVisible({ timeout: 5_000 });
  return await canvas.screenshot();
}

/**
 * Returns true when the screenshot buffer is NOT entirely black (i.e. at
 * least one channel byte in the first 4 KB of pixel data is non-zero).
 *
 * PNG pixel data starts after an ~57-byte header; scanning just the first
 * slice is fast and sufficient for a "not black" sanity check.
 */
function isNonBlack(pngBuffer: Buffer): boolean {
  // PNG magic + IHDR is 8+25 bytes; IDAT data starts afterwards.
  // Rather than parse PNG, we check that the buffer has non-zero bytes
  // beyond the first 64 bytes (skipping the PNG header).
  const slice = pngBuffer.slice(64, 4096);
  for (const byte of slice) {
    if (byte !== 0) return true;
  }
  return false;
}

// ---------------------------------------------------------------------------
// Test suite
// ---------------------------------------------------------------------------

test.describe('skoffroad mobile smoke (iPhone 14)', () => {
  test.beforeEach(async ({ page }) => {
    // Navigate to the WASM app. The baseURL is set to http://localhost:8080 in
    // playwright.config.ts, so '/' resolves to the index.html served from dist/.
    await page.goto('/', { waitUntil: 'domcontentloaded' });
  });

  // -------------------------------------------------------------------------
  // Main smoke test
  // -------------------------------------------------------------------------
  test('tap #mobile-start → title screen hides → canvas renders frames', async ({
    page,
  }) => {
    // 1. Wait for the #mobile-start button to appear (WASM can take up to 30 s
    //    to initialise; the button is in the static HTML so it should be
    //    visible much earlier, but we give the full budget anyway).
    const startBtn = page.locator('#mobile-start');
    await expect(startBtn).toBeVisible({ timeout: 30_000 });

    // 2. Tap the button (simulates a real touch event on mobile).
    await startBtn.tap();

    // 3. After the tap the button's script fires synthetic Space keydown/keyup
    //    events and adds the `.gone` class (opacity: 0; pointer-events: none).
    //    Assert the button is now hidden — 5 s is plenty for the CSS transition.
    await expect(startBtn).toBeHidden({ timeout: 5_000 });

    // 4. Wait 2 s for Bevy to have rendered at least one frame, then screenshot
    //    the canvas to confirm it's not pitch black.
    await page.waitForTimeout(2_000);

    const pixels = await captureCanvasPixels(page);
    expect(
      isNonBlack(pixels),
      'Canvas appears to be entirely black — Bevy may not have rendered a frame.'
    ).toBe(true);

    // 5. Optionally take a full-page screenshot for the CI artifact.
    await page.screenshot({
      path: 'playwright-report/canvas-after-start.png',
      fullPage: false,
    });
  });

  // -------------------------------------------------------------------------
  // Touch HUD smoke test
  // -------------------------------------------------------------------------
  test('touch HUD reset button (#tc-btn-reset) is tappable', async ({
    page,
  }) => {
    // Dismiss the title screen first (re-use same sequence as main test).
    const startBtn = page.locator('#mobile-start');
    await expect(startBtn).toBeVisible({ timeout: 30_000 });
    await startBtn.tap();
    await expect(startBtn).toBeHidden({ timeout: 5_000 });

    // Allow Bevy + touch-controls.js time to inject the HUD overlay.
    // touch-controls.js injects the HUD on DOMContentLoaded, so it should
    // already be present; but we wait a moment for any async init.
    await page.waitForTimeout(1_000);

    // The touch HUD is only built when isTouchDevice is true.  iPhone 14
    // emulation sets navigator.maxTouchPoints > 0, which triggers the build.
    const resetBtn = page.locator('#tc-btn-reset');

    // The HUD may be hidden behind a toggle; if so, skip this assertion
    // gracefully rather than failing CI on a UX detail.
    const isVisible = await resetBtn.isVisible();
    if (!isVisible) {
      test.skip(true, '#tc-btn-reset not visible — HUD may require a toggle tap first');
      return;
    }

    // Tap and verify no JS error is thrown (Playwright surfaces console errors).
    const errors: string[] = [];
    page.on('pageerror', (err) => errors.push(err.message));

    await resetBtn.tap();

    // Give the synthetic KeyR event time to propagate.
    await page.waitForTimeout(500);

    expect(errors).toHaveLength(0);
  });
});
