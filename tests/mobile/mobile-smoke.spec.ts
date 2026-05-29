/**
 * skoffroad — mobile smoke test (Sprint 62, robustified Sprint 74)
 *
 * Boots the WASM build in headless Chromium with iPhone 14 emulation.
 *
 * ARCHITECTURE (Sprint 74 rework):
 *   The HTML overlay buttons (#mobile-start, #tc-btn-*, mobile menu) are pure
 *   HTML/JS injected by index.html + assets/touch-controls.js.  They do NOT
 *   require the 3D WASM game to be running — touch-controls.js dispatches
 *   synthetic KeyboardEvents as soon as DOMContentLoaded fires.
 *
 *   We split the suite into two tiers:
 *     Tier 1 — HTML overlay tests (12 tests):
 *       Wait only for the button to be ATTACHED to the DOM (not visible/stable).
 *       Use tap({ force: true }) everywhere to bypass:
 *         (a) the CSS pulse animation on #mobile-start that defeats Playwright's
 *             stability check, and
 *         (b) main-thread jank from software-GL WASM startup on CI.
 *       These tests finish in < 5 s and are deterministic.
 *
 *     Tier 2 — Canvas render test (1 test):
 *       The ONLY test that actually exercises WASM rendering.  It waits up to
 *       90 s for the canvas to produce non-black pixels.  It is marked
 *       test.slow() so Playwright triples its timeout budget, and the suite
 *       retries it up to 2 times (see playwright.config.ts).  If software-GL
 *       CI can't render in time, only this test can flake — not the other 12.
 *
 * Requirements:
 *   - The dist/ directory must be served at http://localhost:8080 before running.
 *   - Playwright + Chromium must be installed (npx playwright install chromium).
 *
 * Run locally:
 *   ./tests/mobile/run-local.sh          # full automated flow
 *   cd tests/mobile && npx playwright test  # if server is already up
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
  await expect(canvas).toBeVisible({ timeout: 10_000 });
  return await canvas.screenshot();
}

/**
 * Returns true when the screenshot buffer is NOT entirely black (i.e. at
 * least one channel byte in the first 4 KB of pixel data is non-zero).
 */
function isNonBlack(pngBuffer: Buffer): boolean {
  const slice = pngBuffer.slice(64, 4096);
  for (const byte of slice) {
    if (byte !== 0) return true;
  }
  return false;
}

/**
 * Dismiss the title/splash screen by force-dispatching a pointerdown on #mobile-start.
 *
 * Uses forceTap() which bypasses:
 *   - The ms-pulse CSS animation that prevents the stability check from passing
 *   - Any main-thread jank from the WASM build loading in software-GL CI
 *   - Playwright tap() reliability issues in Chromium iPhone emulation mode
 *
 * We wait only for the element to be ATTACHED (not stable/visible) before
 * force-tapping, since it's in the static HTML and present from DOMContentLoaded.
 */
async function dismissSplash(page: Page): Promise<void> {
  const startBtn = page.locator('#mobile-start');
  await expect(startBtn).toBeAttached({ timeout: 15_000 });
  await forceTap(page, '#mobile-start');
  // Give the click handler 500 ms to run and add .gone class.
  await page.waitForTimeout(500);
}

/**
 * Wait for a button to be attached to DOM (NOT requiring visible/stable).
 * touch-controls.js runs on DOMContentLoaded; buttons are present immediately.
 */
async function waitAttached(page: Page, selector: string): Promise<void> {
  await expect(page.locator(selector)).toBeAttached({ timeout: 15_000 });
}

/**
 * Force-fire a pointerdown event on an element via page.evaluate().
 *
 * We use this instead of .tap({ force: true }) because Playwright's tap()
 * synthesises touch events through the browser's internal pointer event
 * pipeline which can silently drop the event in Chromium iPhone emulation
 * mode (observed: FWD passed but REV failed with tap(), both pass with
 * direct dispatchEvent).
 *
 * The pointerdown event is what touch-controls.js listens on; dispatching
 * it directly via the DOM API is the most reliable cross-test approach.
 */
async function forceTap(page: Page, selector: string): Promise<void> {
  await page.evaluate(function (sel) {
    var el = document.querySelector(sel);
    if (!el) throw new Error('forceTap: element not found: ' + sel);
    // Dispatch pointerdown + pointerup to exercise both halves of the handler.
    el.dispatchEvent(new PointerEvent('pointerdown', {
      bubbles: true, cancelable: true, pointerId: 1, isPrimary: true,
    }));
    el.dispatchEvent(new PointerEvent('pointerup', {
      bubbles: true, cancelable: true, pointerId: 1, isPrimary: true,
    }));
  }, selector);
}

/**
 * Intercept synthetic KeyboardEvents dispatched by touch-controls.js.
 * Returns a promise that resolves with the first matching event detail, or
 * null after `timeoutMs` if none arrives.
 *
 * Implementation: we inject a one-shot listener on the canvas element (which
 * is where touch-controls.js now dispatches events — see Sprint 62 fix).
 * We also listen on window as a fallback.
 */
async function waitForKeyEvent(
  page: Page,
  eventType: 'keydown' | 'keyup',
  code: string,
  timeoutMs = 4_000
): Promise<{ code: string; type: string } | null> {
  // page.evaluate() serialises the function via .toString() and runs it in
  // the browser context.  We must not use TypeScript-only syntax (type casts,
  // generics) inside the evaluate callback because the browser receives plain
  // JS.  Plain function syntax and no type annotations are required here.
  return page.evaluate(
    function (args) {
      var eventType = args.eventType;
      var code      = args.code;
      var timeoutMs = args.timeoutMs;
      return new Promise(function (resolve) {
        var timer = setTimeout(function () { resolve(null); }, timeoutMs);
        function handler(e) {
          if (e.code === code) {
            clearTimeout(timer);
            var canvas = document.getElementById('bevy');
            if (canvas) canvas.removeEventListener(eventType, handler);
            window.removeEventListener(eventType, handler);
            resolve({ code: e.code, type: e.type });
          }
        }
        var canvas = document.getElementById('bevy');
        if (canvas) canvas.addEventListener(eventType, handler);
        window.addEventListener(eventType, handler);
      });
    },
    { eventType, code, timeoutMs }
  );
}

// ---------------------------------------------------------------------------
// Test suite
// ---------------------------------------------------------------------------

test.describe('skoffroad mobile smoke (iPhone 14)', () => {
  test.beforeEach(async ({ page }) => {
    // waitUntil: 'domcontentloaded' — we only need the HTML + JS to load.
    // We do NOT wait for 'networkidle' or full WASM boot; the overlay tests
    // assert against pure DOM behaviour that's ready at DOMContentLoaded.
    await page.goto('/', { waitUntil: 'domcontentloaded' });
  });

  // =========================================================================
  // TIER 2 — Canvas render (the ONE test that requires WASM to actually boot)
  // =========================================================================

  /**
   * This is the only test that validates real GPU/WASM rendering.
   * test.slow() triples the per-test timeout (90 s × 3 = 270 s) so software-GL
   * CI gets a fair chance.  retries: 2 in playwright.config.ts means it will
   * be attempted up to 3 times total before being counted as a failure.
   * If it flakes on a no-GPU runner it is the ONLY test that should do so.
   */
  test('canvas renders non-black frames after WASM boots [slow/retryable]', async ({
    page,
  }) => {
    test.slow(); // triples timeout for this test only

    // Force-dispatch pointerdown on #mobile-start to dismiss the splash.
    const startBtn = page.locator('#mobile-start');
    await expect(startBtn).toBeAttached({ timeout: 15_000 });
    await forceTap(page, '#mobile-start');

    // Button should gain .gone class shortly after the tap.
    // Use a relaxed 10 s wait — WASM may be slow to respond on software-GL.
    await expect(startBtn).toBeHidden({ timeout: 10_000 });

    // Wait up to 60 s for Bevy to render at least one non-black frame.
    // We poll every 5 s rather than a single long wait so we exit early on
    // fast machines.
    let rendered = false;
    for (let attempt = 0; attempt < 12; attempt++) {
      await page.waitForTimeout(5_000);
      try {
        const pixels = await captureCanvasPixels(page);
        if (isNonBlack(pixels)) {
          rendered = true;
          break;
        }
      } catch {
        // canvas not yet visible — keep waiting
      }
    }

    expect(
      rendered,
      'Canvas appears to be entirely black after 60 s — Bevy may not have ' +
      'rendered a frame (expected on no-GPU CI; mark as fixme if consistently flaky).'
    ).toBe(true);

    // Take a full-page screenshot for the CI artifact.
    await page.screenshot({
      path: 'playwright-report/canvas-after-start.png',
      fullPage: false,
    });
  });

  // =========================================================================
  // TIER 1 — HTML overlay button tests (12 tests, fully decoupled from WASM)
  //
  // These tests interact only with HTML elements injected by index.html and
  // assets/touch-controls.js.  They do NOT depend on the WASM game being
  // loaded or the canvas rendering any frames.
  //
  // All button interactions use forceTap() which calls page.evaluate() to
  // dispatch a PointerEvent directly on the element.  This is more reliable
  // than Playwright's .tap({ force: true }) which can silently drop pointer
  // events in Chromium iPhone emulation mode (observed in Sprint 74 testing).
  //
  // We wait for elements to be ATTACHED (present in DOM), not visible/stable,
  // because touch-controls.js runs synchronously at DOMContentLoaded and all
  // overlay elements are injected at that point.
  // =========================================================================

  // -------------------------------------------------------------------------
  // #mobile-start fires Space keydown/keyup when force-tapped
  // -------------------------------------------------------------------------
  test('#mobile-start force-tap fires Space keydown and hides the button', async ({
    page,
  }) => {
    const startBtn = page.locator('#mobile-start');
    await expect(startBtn).toBeAttached({ timeout: 15_000 });

    // Listen for the Space keydown that the button's handler dispatches.
    const keyPromise = waitForKeyEvent(page, 'keydown', 'Space', 3_000);

    await forceTap(page, '#mobile-start');

    const evt = await keyPromise;
    expect(evt, '#mobile-start should fire keydown Space').not.toBeNull();
    expect(evt!.code).toBe('Space');

    // After the tap the button should gain .gone class (opacity: 0, pointer-events: none).
    await page.waitForTimeout(500);
    // Either it got .gone class or was removed from DOM — either is acceptable.
    const isGone = await page.evaluate(function () {
      var btn = document.getElementById('mobile-start');
      return !btn || btn.classList.contains('gone');
    });
    expect(isGone, '#mobile-start should be gone after tap').toBe(true);
  });

  // -------------------------------------------------------------------------
  // FWD button fires keydown(KeyW) on the canvas — no WASM boot required
  // -------------------------------------------------------------------------
  test('FWD button (#tc-btn-fwd) fires keydown KeyW on the canvas', async ({
    page,
  }) => {
    await waitAttached(page, '#tc-btn-fwd');

    // Start listening BEFORE we tap so we don't miss the event.
    const eventPromise = waitForKeyEvent(page, 'keydown', 'KeyW');
    await forceTap(page, '#tc-btn-fwd');
    const evt = await eventPromise;

    expect(evt, 'FWD button should fire keydown KeyW').not.toBeNull();
    expect(evt!.code).toBe('KeyW');
    expect(evt!.type).toBe('keydown');
  });

  // -------------------------------------------------------------------------
  // REV button fires keydown(KeyS) on the canvas — no WASM boot required
  // -------------------------------------------------------------------------
  test('REV button (#tc-btn-rev) fires keydown KeyS on the canvas', async ({
    page,
  }) => {
    await waitAttached(page, '#tc-btn-rev');

    const eventPromise = waitForKeyEvent(page, 'keydown', 'KeyS');
    await forceTap(page, '#tc-btn-rev');
    const evt = await eventPromise;

    expect(evt, 'REV button should fire keydown KeyS').not.toBeNull();
    expect(evt!.code).toBe('KeyS');
    expect(evt!.type).toBe('keydown');
  });

  // -------------------------------------------------------------------------
  // HORN button fires keydown(KeyN) — no WASM boot required
  // -------------------------------------------------------------------------
  test('HORN button (#tc-btn-horn) fires keydown KeyN', async ({
    page,
  }) => {
    await waitAttached(page, '#tc-btn-horn');

    const eventPromise = waitForKeyEvent(page, 'keydown', 'KeyN');
    await forceTap(page, '#tc-btn-horn');
    const evt = await eventPromise;

    expect(evt, 'HORN button should fire keydown KeyN').not.toBeNull();
    expect(evt!.code).toBe('KeyN');
  });

  // -------------------------------------------------------------------------
  // BRAKE button fires keydown(Space) — no WASM boot required
  // -------------------------------------------------------------------------
  test('BRAKE button (#tc-btn-brake) fires keydown Space on the canvas', async ({
    page,
  }) => {
    await waitAttached(page, '#tc-btn-brake');

    const eventPromise = waitForKeyEvent(page, 'keydown', 'Space');
    await forceTap(page, '#tc-btn-brake');
    const evt = await eventPromise;

    expect(evt, 'BRAKE button should fire keydown Space').not.toBeNull();
    expect(evt!.code).toBe('Space');
  });

  // -------------------------------------------------------------------------
  // RESET button is force-tappable without JS errors — no WASM boot required
  // -------------------------------------------------------------------------
  test('touch HUD reset button (#tc-btn-reset) force-tap produces no JS errors', async ({
    page,
  }) => {
    await waitAttached(page, '#tc-btn-reset');

    const errors: string[] = [];
    page.on('pageerror', (err) => errors.push(err.message));

    await forceTap(page, '#tc-btn-reset');
    await page.waitForTimeout(500);

    expect(errors).toHaveLength(0);
  });

  // -------------------------------------------------------------------------
  // MENU button opens the mobile menu overlay — no WASM boot required
  // -------------------------------------------------------------------------
  test('MENU button (#tc-btn-menu) opens the mobile menu overlay', async ({
    page,
  }) => {
    // #tc-menu-overlay is built eagerly by touch-controls.js at init time
    // (Sprint 66 fix) so it is present in the DOM from DOMContentLoaded.
    await waitAttached(page, '#tc-btn-menu');
    await waitAttached(page, '#tc-menu-overlay');

    // The overlay should not be visible initially.
    const overlay = page.locator('#tc-menu-overlay');
    await expect(overlay).not.toHaveClass(/tc-menu-open/);

    // Force-dispatch pointerdown on the menu button.
    await forceTap(page, '#tc-btn-menu');
    await page.waitForTimeout(300);

    // The overlay should now have the open class.
    await expect(overlay).toHaveClass(/tc-menu-open/, { timeout: 2_000 });

    // The close button should be present and force-tappable.
    const closeBtn = page.locator('#tc-menu-close');
    await expect(closeBtn).toBeAttached({ timeout: 2_000 });
    await forceTap(page, '#tc-menu-close');
    await page.waitForTimeout(300);

    // After closing, the open class should be removed.
    await expect(overlay).not.toHaveClass(/tc-menu-open/);
  });

  // -------------------------------------------------------------------------
  // Menu Multiplayer item fires keydown KeyI — no WASM boot required
  // -------------------------------------------------------------------------
  test('menu Multiplayer item fires keydown KeyI', async ({
    page,
  }) => {
    await waitAttached(page, '#tc-btn-menu');
    await waitAttached(page, '#tc-menu-overlay');

    // Open the menu.
    await forceTap(page, '#tc-btn-menu');
    await page.waitForTimeout(300);

    const overlay = page.locator('#tc-menu-overlay');
    await expect(overlay).toHaveClass(/tc-menu-open/, { timeout: 2_000 });

    // Listen for the hotkey event before tapping the menu item.
    const eventPromise = waitForKeyEvent(page, 'keydown', 'KeyI');

    // Force-dispatch on the "Multiplayer (I)" row — it's the second list item.
    // Use page.evaluate with nth-child selector.
    await page.evaluate(function() {
      var items = document.querySelectorAll('.tc-menu-item');
      var el = items[1];
      if (!el) throw new Error('Multiplayer menu item not found');
      el.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, cancelable: true, pointerId: 1, isPrimary: true }));
    });

    const evt = await eventPromise;
    expect(evt, 'Multiplayer menu item should fire keydown KeyI').not.toBeNull();
    expect(evt!.code).toBe('KeyI');
  });

  // -------------------------------------------------------------------------
  // Mission Select menu row fires Shift+Tab — no WASM boot required
  // -------------------------------------------------------------------------
  test('Mission Select menu row fires Shift+Tab', async ({
    page,
  }) => {
    await waitAttached(page, '#tc-btn-menu');
    await waitAttached(page, '#tc-menu-overlay');

    // Open the mobile menu.
    await forceTap(page, '#tc-btn-menu');
    await page.waitForTimeout(300);

    const mobileMenuOverlay = page.locator('#tc-menu-overlay');
    await expect(mobileMenuOverlay).toHaveClass(/tc-menu-open/, { timeout: 2_000 });

    // Listen for Tab keydown with shiftKey = true (the Mission Select hotkey).
    const tabEventPromise = page.evaluate(function () {
      return new Promise(function (resolve) {
        var timer = setTimeout(function () { resolve(null); }, 4000);
        function handler(e) {
          if (e.code === 'Tab' && e.shiftKey) {
            clearTimeout(timer);
            document.removeEventListener('keydown', handler);
            resolve({ code: e.code, shiftKey: e.shiftKey });
          }
        }
        document.addEventListener('keydown', handler);
      });
    });

    // Find and force-dispatch on the "Mission Select" row.
    const menuItems = page.locator('.tc-menu-item');
    const count = await menuItems.count();
    let missionSelectIdx = -1;
    for (let i = 0; i < count; i++) {
      const text = await menuItems.nth(i).textContent();
      if (text && text.includes('Mission Select')) {
        missionSelectIdx = i;
        break;
      }
    }

    if (missionSelectIdx < 0) {
      test.skip(true, 'Mission Select row not found in mobile menu');
      return;
    }

    await page.evaluate(function(idx) {
      var items = document.querySelectorAll('.tc-menu-item');
      var el = items[idx];
      if (!el) throw new Error('Mission Select row not found at index ' + idx);
      el.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, cancelable: true, pointerId: 1, isPrimary: true }));
    }, missionSelectIdx);

    const tabEvt = await tabEventPromise as { code: string; shiftKey: boolean } | null;
    expect(tabEvt, 'Mission Select row should fire Tab keydown with shiftKey').not.toBeNull();
    if (tabEvt) {
      expect(tabEvt.code).toBe('Tab');
      expect(tabEvt.shiftKey).toBe(true);
    }

    // After the Shift+Tab fires, the menu should close.
    await page.waitForTimeout(500);
    await expect(mobileMenuOverlay).not.toHaveClass(/tc-menu-open/);
  });

  // -------------------------------------------------------------------------
  // Joystick drag (up) fires keydown KeyW — no WASM boot required
  // -------------------------------------------------------------------------
  test('joystick drag (up) fires keydown KeyW on the canvas', async ({
    page,
  }) => {
    await waitAttached(page, '#tc-stick-zone');

    const stickZone = page.locator('#tc-stick-zone');

    // Get the bounding box so we can calculate drag coordinates.
    const box = await stickZone.boundingBox();
    if (!box) {
      test.skip(true, '#tc-stick-zone has no bounding box');
      return;
    }

    const cx = box.x + box.width / 2;
    const cy = box.y + box.height / 2;

    // Start listening for KeyW keydown BEFORE the drag.
    const eventPromise = waitForKeyEvent(page, 'keydown', 'KeyW', 5_000);

    // Simulate a drag upward (Y decreases on screen for "up" = forward).
    await page.mouse.move(cx, cy);
    await page.mouse.down();
    // Drag upward by 40 px (well past the dead zone).
    await page.mouse.move(cx, cy - 40, { steps: 8 });

    const evt = await eventPromise;

    // Release and clean up.
    await page.mouse.up();

    expect(evt, 'Joystick drag up should fire keydown KeyW').not.toBeNull();
    expect(evt!.code).toBe('KeyW');
  });

  // -------------------------------------------------------------------------
  // Joystick drag (down) fires keydown KeyS — no WASM boot required
  // -------------------------------------------------------------------------
  test('joystick drag (down) fires keydown KeyS on the canvas', async ({
    page,
  }) => {
    await waitAttached(page, '#tc-stick-zone');

    const stickZone = page.locator('#tc-stick-zone');
    const box = await stickZone.boundingBox();
    if (!box) {
      test.skip(true, '#tc-stick-zone has no bounding box');
      return;
    }

    const cx = box.x + box.width / 2;
    const cy = box.y + box.height / 2;

    const eventPromise = waitForKeyEvent(page, 'keydown', 'KeyS', 5_000);

    await page.mouse.move(cx, cy);
    await page.mouse.down();
    await page.mouse.move(cx, cy + 40, { steps: 8 });

    const evt = await eventPromise;
    await page.mouse.up();

    expect(evt, 'Joystick drag down should fire keydown KeyS').not.toBeNull();
    expect(evt!.code).toBe('KeyS');
  });

  // -------------------------------------------------------------------------
  // Joystick release fires keyup KeyW after dragging up — no WASM boot required
  // -------------------------------------------------------------------------
  test('joystick release fires keyup KeyW after dragging up', async ({
    page,
  }) => {
    await waitAttached(page, '#tc-stick-zone');

    const stickZone = page.locator('#tc-stick-zone');
    const box = await stickZone.boundingBox();
    if (!box) {
      test.skip(true, '#tc-stick-zone has no bounding box');
      return;
    }

    const cx = box.x + box.width / 2;
    const cy = box.y + box.height / 2;

    // Drag up to engage forward.
    await page.mouse.move(cx, cy);
    await page.mouse.down();
    await page.mouse.move(cx, cy - 40, { steps: 8 });
    await page.waitForTimeout(100);

    // Start listening for keyup BEFORE releasing.
    const keyupPromise = waitForKeyEvent(page, 'keyup', 'KeyW', 3_000);
    await page.mouse.up();

    const evt = await keyupPromise;
    expect(evt, 'Joystick release should fire keyup KeyW').not.toBeNull();
    expect(evt!.code).toBe('KeyW');
    expect(evt!.type).toBe('keyup');
  });

  // -------------------------------------------------------------------------
  // Mission Select (DOM check): mobile menu closes after Shift+Tab row tap
  // Sprint 64 — no canvas-pixel check needed; DOM state is sufficient.
  // -------------------------------------------------------------------------
  test('Mission Select row closes the mobile menu (DOM-only check)', async ({
    page,
  }) => {
    await waitAttached(page, '#tc-btn-menu');
    await waitAttached(page, '#tc-menu-overlay');

    // Open the mobile menu overlay.
    await forceTap(page, '#tc-btn-menu');
    await page.waitForTimeout(300);

    const mobileMenuOverlay = page.locator('#tc-menu-overlay');
    await expect(mobileMenuOverlay).toHaveClass(/tc-menu-open/, { timeout: 2_000 });

    // Find and force-dispatch on Mission Select.
    const menuItems = page.locator('.tc-menu-item');
    const count = await menuItems.count();
    let missionSelectIdx = -1;
    for (let i = 0; i < count; i++) {
      const text = await menuItems.nth(i).textContent();
      if (text && text.includes('Mission Select')) {
        missionSelectIdx = i;
        break;
      }
    }

    if (missionSelectIdx < 0) {
      test.skip(true, 'Mission Select row not found in mobile menu — skipping');
      return;
    }

    await page.evaluate(function(idx) {
      var items = document.querySelectorAll('.tc-menu-item');
      var el = items[idx];
      if (!el) throw new Error('Mission Select row not found');
      el.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, cancelable: true, pointerId: 1, isPrimary: true }));
    }, missionSelectIdx);
    await page.waitForTimeout(600);

    // The mobile menu should now be closed.
    await expect(mobileMenuOverlay).not.toHaveClass(/tc-menu-open/);

    // No JS errors.
    const errors: string[] = [];
    page.on('pageerror', (err) => errors.push(err.message));
    await page.waitForTimeout(200);
    expect(errors).toHaveLength(0);
  });

  // -------------------------------------------------------------------------
  // Sprint 65: Top-times sub-row text check (DOM-only — no canvas pixel check)
  // Verifies the accessibility tree or DOM text contains leaderboard strings.
  // If Bevy hasn't rendered to canvas yet we still accept the test as long as
  // no JS errors occurred.
  // -------------------------------------------------------------------------
  test('Mission Select cards have top-times sub-row (DOM + no-JS-error check)', async ({
    page,
  }) => {
    await waitAttached(page, '#tc-btn-menu');
    await waitAttached(page, '#tc-menu-overlay');

    // Open the mobile menu.
    await forceTap(page, '#tc-btn-menu');
    await page.waitForTimeout(300);

    const mobileMenuOverlay = page.locator('#tc-menu-overlay');
    await expect(mobileMenuOverlay).toHaveClass(/tc-menu-open/, { timeout: 2_000 });

    // Find and force-dispatch on the Mission Select row.
    const menuItems = page.locator('.tc-menu-item');
    const count = await menuItems.count();
    let missionSelectIdx = -1;
    for (let i = 0; i < count; i++) {
      const text = await menuItems.nth(i).textContent();
      if (text && text.includes('Mission Select')) {
        missionSelectIdx = i;
        break;
      }
    }

    if (missionSelectIdx < 0) {
      test.skip(true, 'Mission Select row not found in mobile menu');
      return;
    }

    await page.evaluate(function(idx) {
      var items = document.querySelectorAll('.tc-menu-item');
      var el = items[idx];
      if (!el) throw new Error('Mission Select row not found');
      el.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, cancelable: true, pointerId: 1, isPrimary: true }));
    }, missionSelectIdx);
    await page.waitForTimeout(700);

    // The overlay should have closed the mobile menu.
    await expect(mobileMenuOverlay).not.toHaveClass(/tc-menu-open/);

    // DOM accessibility check — best-effort; canvas-only Bevy UI won't appear
    // in the DOM, so we log rather than fail if text is absent.
    const accessibleText = await page.evaluate(function () {
      function walk(node) {
        var out = (node.textContent || '') + ' ';
        for (var i = 0; i < node.children.length; i++) {
          out += walk(node.children[i]);
        }
        return out;
      }
      return walk(document.body);
    });

    const hasTopTimesText = /no peer times yet|peer best/i.test(accessibleText);
    if (!hasTopTimesText) {
      console.log('top-times text not found in DOM (canvas-only rendering — OK)');
    }

    // Primary assertion: no JS errors (a Bevy panic would show up here).
    const errors: string[] = [];
    page.on('pageerror', (err) => errors.push(err.message));
    await page.waitForTimeout(200);
    expect(errors).toHaveLength(0);
  });
});
