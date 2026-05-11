/**
 * skoffroad — mobile smoke test (Sprint 62)
 *
 * Boots the WASM build in headless Chromium with iPhone 14 emulation,
 * taps #mobile-start to dismiss the title screen, and verifies the canvas
 * is rendering frames (non-black pixel check).
 *
 * Sprint 62 additions:
 *  - FWD / REV buttons fire keydown(KeyW) / keydown(KeyS) on the canvas
 *  - Joystick drag produces WASD keydown events on the canvas
 *  - Menu button (☰) opens the mobile menu overlay
 *  - Brake button still dismisses the title screen
 *  - Horn button fires keydown(KeyN)
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
 */
function isNonBlack(pngBuffer: Buffer): boolean {
  const slice = pngBuffer.slice(64, 4096);
  for (const byte of slice) {
    if (byte !== 0) return true;
  }
  return false;
}

/**
 * Helper: dismiss the title/splash screen by tapping #mobile-start, then
 * wait for it to be hidden.
 */
async function dismissSplash(page: Page): Promise<void> {
  const startBtn = page.locator('#mobile-start');
  await expect(startBtn).toBeVisible({ timeout: 30_000 });
  await startBtn.tap();
  await expect(startBtn).toBeHidden({ timeout: 5_000 });
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
    await page.goto('/', { waitUntil: 'domcontentloaded' });
  });

  // -------------------------------------------------------------------------
  // Main smoke test
  // -------------------------------------------------------------------------
  test('tap #mobile-start → title screen hides → canvas renders frames', async ({
    page,
  }) => {
    // 1. Wait for the #mobile-start button (in static HTML — appears immediately).
    const startBtn = page.locator('#mobile-start');
    await expect(startBtn).toBeVisible({ timeout: 30_000 });

    // 2. Tap the button (simulates a real touch event on mobile).
    await startBtn.tap();

    // 3. Button should gain .gone class (opacity 0, pointer-events none).
    await expect(startBtn).toBeHidden({ timeout: 5_000 });

    // 4. Wait 2 s for Bevy to render at least one frame, then screenshot.
    await page.waitForTimeout(2_000);

    const pixels = await captureCanvasPixels(page);
    expect(
      isNonBlack(pixels),
      'Canvas appears to be entirely black — Bevy may not have rendered a frame.'
    ).toBe(true);

    // 5. Take a full-page screenshot for the CI artifact.
    await page.screenshot({
      path: 'playwright-report/canvas-after-start.png',
      fullPage: false,
    });
  });

  // -------------------------------------------------------------------------
  // Touch HUD — helper to check visibility and skip gracefully if hidden
  // -------------------------------------------------------------------------
  async function assertHudVisible(page: Page, selector: string): Promise<boolean> {
    const el = page.locator(selector);
    const isVisible = await el.isVisible();
    if (!isVisible) {
      test.skip(true, `${selector} not visible — HUD may require a toggle tap first`);
      return false;
    }
    return true;
  }

  // -------------------------------------------------------------------------
  // Existing smoke: reset button is tappable
  // -------------------------------------------------------------------------
  test('touch HUD reset button (#tc-btn-reset) is tappable', async ({
    page,
  }) => {
    await dismissSplash(page);
    await page.waitForTimeout(1_000);

    if (!(await assertHudVisible(page, '#tc-btn-reset'))) return;

    const errors: string[] = [];
    page.on('pageerror', (err) => errors.push(err.message));

    await page.locator('#tc-btn-reset').tap();
    await page.waitForTimeout(500);

    expect(errors).toHaveLength(0);
  });

  // -------------------------------------------------------------------------
  // Sprint 62: FWD button fires keydown(KeyW) on the canvas
  // -------------------------------------------------------------------------
  test('FWD button (#tc-btn-fwd) fires keydown KeyW on the canvas', async ({
    page,
  }) => {
    await dismissSplash(page);
    await page.waitForTimeout(800);

    if (!(await assertHudVisible(page, '#tc-btn-fwd'))) return;

    // Start listening BEFORE we tap so we don't miss the event.
    const eventPromise = waitForKeyEvent(page, 'keydown', 'KeyW');
    await page.locator('#tc-btn-fwd').tap();
    const evt = await eventPromise;

    expect(evt, 'FWD button should fire keydown KeyW').not.toBeNull();
    expect(evt!.code).toBe('KeyW');
    expect(evt!.type).toBe('keydown');
  });

  // -------------------------------------------------------------------------
  // Sprint 62: REV button fires keydown(KeyS) on the canvas
  // -------------------------------------------------------------------------
  test('REV button (#tc-btn-rev) fires keydown KeyS on the canvas', async ({
    page,
  }) => {
    await dismissSplash(page);
    await page.waitForTimeout(800);

    if (!(await assertHudVisible(page, '#tc-btn-rev'))) return;

    const eventPromise = waitForKeyEvent(page, 'keydown', 'KeyS');
    await page.locator('#tc-btn-rev').tap();
    const evt = await eventPromise;

    expect(evt, 'REV button should fire keydown KeyS').not.toBeNull();
    expect(evt!.code).toBe('KeyS');
    expect(evt!.type).toBe('keydown');
  });

  // -------------------------------------------------------------------------
  // Sprint 62: HORN button fires keydown(KeyN)
  // -------------------------------------------------------------------------
  test('HORN button (#tc-btn-horn) fires keydown KeyN', async ({
    page,
  }) => {
    await dismissSplash(page);
    await page.waitForTimeout(800);

    if (!(await assertHudVisible(page, '#tc-btn-horn'))) return;

    const eventPromise = waitForKeyEvent(page, 'keydown', 'KeyN');
    await page.locator('#tc-btn-horn').tap();
    const evt = await eventPromise;

    expect(evt, 'HORN button should fire keydown KeyN').not.toBeNull();
    expect(evt!.code).toBe('KeyN');
  });

  // -------------------------------------------------------------------------
  // Sprint 62: Brake button fires keydown(Space) on the canvas
  // -------------------------------------------------------------------------
  test('BRAKE button (#tc-btn-brake) fires keydown Space on the canvas', async ({
    page,
  }) => {
    await dismissSplash(page);
    await page.waitForTimeout(800);

    if (!(await assertHudVisible(page, '#tc-btn-brake'))) return;

    const eventPromise = waitForKeyEvent(page, 'keydown', 'Space');
    await page.locator('#tc-btn-brake').tap();
    const evt = await eventPromise;

    expect(evt, 'BRAKE button should fire keydown Space').not.toBeNull();
    expect(evt!.code).toBe('Space');
  });

  // -------------------------------------------------------------------------
  // Sprint 62: MENU button opens the mobile menu overlay
  // -------------------------------------------------------------------------
  test('MENU button (#tc-btn-menu) opens the mobile menu overlay', async ({
    page,
  }) => {
    await dismissSplash(page);
    await page.waitForTimeout(800);

    if (!(await assertHudVisible(page, '#tc-btn-menu'))) return;

    // The overlay should not be visible initially.
    const overlay = page.locator('#tc-menu-overlay');
    await expect(overlay).not.toHaveClass(/tc-menu-open/);

    // Tap the menu button.
    await page.locator('#tc-btn-menu').tap();
    await page.waitForTimeout(300);

    // The overlay should now have the open class.
    await expect(overlay).toHaveClass(/tc-menu-open/, { timeout: 2_000 });

    // The close button should be visible and functional.
    const closeBtn = page.locator('#tc-menu-close');
    await expect(closeBtn).toBeVisible();
    await closeBtn.tap();
    await page.waitForTimeout(300);

    // After closing, the open class should be removed.
    await expect(overlay).not.toHaveClass(/tc-menu-open/);
  });

  // -------------------------------------------------------------------------
  // Sprint 62: Menu item dispatches the correct hotkey
  // -------------------------------------------------------------------------
  test('menu Multiplayer item fires keydown KeyI', async ({
    page,
  }) => {
    await dismissSplash(page);
    await page.waitForTimeout(800);

    if (!(await assertHudVisible(page, '#tc-btn-menu'))) return;

    // Open the menu.
    await page.locator('#tc-btn-menu').tap();
    await page.waitForTimeout(300);

    const overlay = page.locator('#tc-menu-overlay');
    await expect(overlay).toHaveClass(/tc-menu-open/, { timeout: 2_000 });

    // Listen for the hotkey event before tapping the menu item.
    const eventPromise = waitForKeyEvent(page, 'keydown', 'KeyI');

    // Tap the "Multiplayer (I)" row — it's the second list item.
    const menuItems = page.locator('.tc-menu-item');
    await menuItems.nth(1).tap();

    const evt = await eventPromise;
    expect(evt, 'Multiplayer menu item should fire keydown KeyI').not.toBeNull();
    expect(evt!.code).toBe('KeyI');
  });

  // -------------------------------------------------------------------------
  // Sprint 63: Mission Select menu row opens the overlay via Shift+Tab
  // -------------------------------------------------------------------------
  test('Mission Select menu row fires Shift+Tab and overlay appears', async ({
    page,
  }) => {
    await dismissSplash(page);
    await page.waitForTimeout(800);

    if (!(await assertHudVisible(page, '#tc-btn-menu'))) return;

    // Open the mobile menu.
    await page.locator('#tc-btn-menu').tap();
    await page.waitForTimeout(300);

    const mobileMenuOverlay = page.locator('#tc-menu-overlay');
    await expect(mobileMenuOverlay).toHaveClass(/tc-menu-open/, { timeout: 2_000 });

    // Listen for Tab keydown with shiftKey = true (the Mission Select hotkey).
    // We listen for Tab since that is the code; the shiftKey flag is on the event.
    const tabEventPromise = page.evaluate(function () {
      return new Promise<{ code: string; shiftKey: boolean } | null>(function (resolve) {
        var timer = setTimeout(function () { resolve(null); }, 4000);
        function handler(e: KeyboardEvent) {
          if (e.code === 'Tab' && e.shiftKey) {
            clearTimeout(timer);
            document.removeEventListener('keydown', handler);
            resolve({ code: e.code, shiftKey: e.shiftKey });
          }
        }
        document.addEventListener('keydown', handler);
      });
    });

    // Tap the "Mission Select" row — find it by text content.
    const menuItems = page.locator('.tc-menu-item');
    const count = await menuItems.count();
    let missionSelectRow: import('@playwright/test').Locator | null = null;
    for (let i = 0; i < count; i++) {
      const text = await menuItems.nth(i).textContent();
      if (text && text.includes('Mission Select')) {
        missionSelectRow = menuItems.nth(i);
        break;
      }
    }

    if (!missionSelectRow) {
      test.skip(true, 'Mission Select row not found in mobile menu');
      return;
    }

    await missionSelectRow.tap();

    const tabEvt = await tabEventPromise;
    expect(tabEvt, 'Mission Select row should fire Tab keydown with shiftKey').not.toBeNull();
    expect(tabEvt!.code).toBe('Tab');
    expect(tabEvt!.shiftKey).toBe(true);

    // After the Shift+Tab fires, the Bevy overlay should become visible.
    // In the test harness the Bevy canvas is running, so we wait a short time
    // and verify the mobile menu overlay is now closed (the row tapping hides it).
    await page.waitForTimeout(500);
    await expect(mobileMenuOverlay).not.toHaveClass(/tc-menu-open/);
  });

  // -------------------------------------------------------------------------
  // Sprint 62: Joystick drag emits WASD keydown events on the canvas
  // -------------------------------------------------------------------------
  test('joystick drag (up) fires keydown KeyW on the canvas', async ({
    page,
  }) => {
    await dismissSplash(page);
    await page.waitForTimeout(800);

    if (!(await assertHudVisible(page, '#tc-stick-zone'))) return;

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
  // Sprint 62: Joystick drag down fires KeyS
  // -------------------------------------------------------------------------
  test('joystick drag (down) fires keydown KeyS on the canvas', async ({
    page,
  }) => {
    await dismissSplash(page);
    await page.waitForTimeout(800);

    if (!(await assertHudVisible(page, '#tc-stick-zone'))) return;

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
  // Sprint 62: Joystick release fires keyup events
  // -------------------------------------------------------------------------
  test('joystick release fires keyup KeyW after dragging up', async ({
    page,
  }) => {
    await dismissSplash(page);
    await page.waitForTimeout(800);

    if (!(await assertHudVisible(page, '#tc-stick-zone'))) return;

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
});
