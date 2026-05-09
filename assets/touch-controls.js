/**
 * skoffroad – mobile touch controls
 * Sprint 52 — v0.15.0
 *
 * Strategy: synthesise KeyboardEvent dispatches on `window` so that Bevy's
 * winit input layer (which reads DOM keyboard events in WASM builds) sees
 * exactly the same events a physical keyboard would produce.
 *
 * This keeps the Rust codebase completely unchanged — drive_input_keyboard
 * continues to work as-is.
 */

(function () {
  'use strict';

  /* ── Helpers ──────────────────────────────────────────────── */

  /** Dispatch a synthetic KeyboardEvent on window. */
  function fireKey(type, code, key) {
    // Bevy/winit reads both `code` (physical) and `key` (logical).
    window.dispatchEvent(
      new KeyboardEvent(type, {
        code: code,
        key: key || code,
        bubbles: true,
        cancelable: true,
      })
    );
  }

  function keyDown(code, key) { fireKey('keydown', code, key); }
  function keyUp(code, key)   { fireKey('keyup',   code, key); }

  /* ── Touch detection ──────────────────────────────────────── */

  const params       = new URLSearchParams(window.location.search);
  const forceTouch   = params.has('force-touch');
  const isTouchDevice =
    forceTouch ||
    'ontouchstart' in window ||
    navigator.maxTouchPoints > 0;

  /* ── Build DOM ────────────────────────────────────────────── */

  /**
   * Build the full touch-controls overlay and inject it into <body>.
   * Returns the root element so the toggle can show/hide it.
   */
  function buildOverlay() {
    // ── Root ────────────────────────────────────────────────
    const root = document.createElement('div');
    root.id = 'touch-controls';

    // ── Joystick ────────────────────────────────────────────
    const stickZone = document.createElement('div');
    stickZone.id = 'tc-stick-zone';

    const stickRing = document.createElement('div');
    stickRing.id = 'tc-stick-ring';

    const stickThumb = document.createElement('div');
    stickThumb.id = 'tc-stick-thumb';

    stickRing.appendChild(stickThumb);
    stickZone.appendChild(stickRing);
    root.appendChild(stickZone);

    // ── Button grid ─────────────────────────────────────────
    const buttons = document.createElement('div');
    buttons.id = 'tc-buttons';

    /**
     * @param {string} id       element id
     * @param {string} icon     emoji / unicode icon
     * @param {string} label    short text label
     * @param {string} code     KeyboardEvent.code to synthesise
     * @param {string} [key]    KeyboardEvent.key (defaults to code)
     */
    function makeBtn(id, icon, label, code, key) {
      const btn = document.createElement('button');
      btn.id = id;
      btn.className = 'tc-btn';
      btn.setAttribute('aria-label', label);
      btn.type = 'button';

      const iconEl = document.createElement('span');
      iconEl.className = 'tc-btn-icon';
      iconEl.textContent = icon;

      const labelEl = document.createElement('span');
      labelEl.textContent = label;

      btn.appendChild(iconEl);
      btn.appendChild(labelEl);

      // All buttons: keydown on pointerdown, keyup on release/cancel/leave.
      // PTT (F, Brake) and momentary (R, V, P…) both use this pattern —
      // the distinction is just semantic; the event flow is identical.
      btn.addEventListener('pointerdown', function (e) {
        e.preventDefault();
        btn.setPointerCapture(e.pointerId);
        btn.classList.add('tc-pressed');
        keyDown(code, key || code);
      });
      const releaseBtn = function (e) {
        e.preventDefault();
        btn.classList.remove('tc-pressed');
        keyUp(code, key || code);
      };
      btn.addEventListener('pointerup',     releaseBtn);
      btn.addEventListener('pointercancel', releaseBtn);

      return btn;
    }

    // Row 1: Reset | Camera | Photo
    buttons.appendChild(makeBtn('tc-btn-reset',  '🔄', 'R',     'KeyR',    'r'));
    buttons.appendChild(makeBtn('tc-btn-cam',    '📷', 'V',     'KeyV',    'v'));
    buttons.appendChild(makeBtn('tc-btn-photo',  '🖼️', 'P',     'KeyP',    'p'));

    // Row 2: Multiplayer | PTT | Pause
    buttons.appendChild(makeBtn('tc-btn-mp',     '🌐', 'I',     'KeyI',    'i'));
    const pttBtn = makeBtn('tc-btn-ptt', '🎙️', 'F', 'KeyF', 'f');
    buttons.appendChild(pttBtn);
    buttons.appendChild(makeBtn('tc-btn-esc',    '⏸', 'Esc',   'Escape',  'Escape'));

    // Row 3: Brake (full-width — hold to brake)
    const brakeBtn = makeBtn('tc-btn-brake', '🛑', 'BRAKE', 'Space', ' ');
    buttons.appendChild(brakeBtn);

    root.appendChild(buttons);

    document.body.appendChild(root);
    return root;
  }

  /* ── Desktop toggle pill ──────────────────────────────────── */

  function buildToggle(overlayRoot) {
    const pill = document.createElement('button');
    pill.id = 'tc-toggle';
    pill.type = 'button';
    pill.textContent = 'touch HUD';
    document.body.appendChild(pill);

    let visible = isTouchDevice;
    if (!visible) overlayRoot.classList.add('tc-hidden');

    pill.addEventListener('click', function () {
      visible = !visible;
      overlayRoot.classList.toggle('tc-hidden', !visible);
      pill.textContent = visible ? 'hide touch HUD' : 'touch HUD';
    });
  }

  /* ── Joystick logic ───────────────────────────────────────── */

  /** Keys currently held down by the joystick (to avoid redundant events). */
  const stickHeld = { KeyW: false, KeyS: false, KeyA: false, KeyD: false };

  function setStickKey(code, key, shouldHold) {
    if (shouldHold === stickHeld[code]) return; // no change
    stickHeld[code] = shouldHold;
    if (shouldHold) {
      keyDown(code, key);
    } else {
      keyUp(code, key);
    }
  }

  function releaseAllStick() {
    setStickKey('KeyW', 'w', false);
    setStickKey('KeyS', 's', false);
    setStickKey('KeyA', 'a', false);
    setStickKey('KeyD', 'd', false);
  }

  /**
   * Given a normalised joystick position (nx, ny) in [-1, 1],
   * update WASD held state.
   *
   * Axis conventions (matching drive_input_keyboard):
   *   +Y (thumb up)   → W (forward / throttle)
   *   -Y (thumb down) → S (reverse)
   *   -X (thumb left) → A (steer left)
   *   +X (thumb right)→ D (steer right)
   *
   * Dead-zone threshold: 0.25
   */
  const DEAD = 0.25;

  function applyStick(nx, ny) {
    setStickKey('KeyW', 'w',  ny > DEAD);
    setStickKey('KeyS', 's', -ny > DEAD);
    setStickKey('KeyA', 'a', -nx > DEAD);
    setStickKey('KeyD', 'd',  nx > DEAD);
  }

  function initJoystick(overlayRoot) {
    const zone  = overlayRoot.querySelector('#tc-stick-zone');
    const thumb = overlayRoot.querySelector('#tc-stick-thumb');

    // Outer ring radius (px) — half of the zone's 140 px width minus thumb half
    const RING_R  = 70;   // px — outer ring radius
    const THUMB_R = 30;   // px — thumb radius (half of 60 px)
    const MAX_R   = RING_R - THUMB_R;  // max thumb travel from centre

    let activeTouchId = null;
    let originX = 0;
    let originY = 0;

    function getZoneCenter() {
      const rect = zone.getBoundingClientRect();
      return { cx: rect.left + rect.width / 2, cy: rect.top + rect.height / 2 };
    }

    function updateThumb(nx, ny) {
      // nx, ny in [-1, 1]
      const dx = nx * MAX_R;
      const dy = -ny * MAX_R; // screen Y is inverted
      thumb.style.transform =
        'translate(calc(-50% + ' + dx + 'px), calc(-50% + ' + dy + 'px))';
    }

    function resetThumb() {
      thumb.style.transform = 'translate(-50%, -50%)';
      thumb.style.transition = 'transform 0.15s ease';
      setTimeout(function () { thumb.style.transition = ''; }, 160);
    }

    zone.addEventListener('pointerdown', function (e) {
      if (activeTouchId !== null) return; // single-touch only
      e.preventDefault();
      zone.setPointerCapture(e.pointerId);
      activeTouchId = e.pointerId;
      const { cx, cy } = getZoneCenter();
      originX = cx;
      originY = cy;
    });

    zone.addEventListener('pointermove', function (e) {
      if (e.pointerId !== activeTouchId) return;
      e.preventDefault();

      // Screen-space delta from zone centre (screen Y increases downward).
      const sdx = e.clientX - originX;
      const sdy = e.clientY - originY;
      const dist = Math.sqrt(sdx * sdx + sdy * sdy);
      const clamped = Math.min(dist, MAX_R);
      const angle = Math.atan2(sdy, sdx); // atan2 in screen space

      // Normalised screen-space components [-1, 1].
      const nsx = clamped === 0 ? 0 : (Math.cos(angle) * clamped) / MAX_R;
      const nsy = clamped === 0 ? 0 : (Math.sin(angle) * clamped) / MAX_R; // +ve = down on screen

      // Logical-space Y: invert so +ve = up (forward drive).
      const nx = nsx;
      const ny = -nsy;

      // updateThumb expects logical-Y (converts back to screen-Y internally).
      updateThumb(nx, ny);
      applyStick(nx, ny);
    });

    function onRelease(e) {
      if (e.pointerId !== activeTouchId) return;
      e.preventDefault();
      activeTouchId = null;
      resetThumb();
      releaseAllStick();
    }

    zone.addEventListener('pointerup',     onRelease);
    zone.addEventListener('pointercancel', onRelease);
  }

  /* ── Init ─────────────────────────────────────────────────── */

  function init() {
    // Always build the overlay; show/hide via CSS class.
    // On non-touch desktop it starts hidden unless ?force-touch=1.
    const overlayRoot = buildOverlay();
    buildToggle(overlayRoot);
    initJoystick(overlayRoot);

    if (isTouchDevice) {
      // Prevent the browser's default scroll/zoom on the canvas so touch
      // events reach our handlers cleanly.
      const canvas = document.getElementById('bevy');
      if (canvas) {
        canvas.style.touchAction = 'none';
      }
    }

    // Console hint for dev verification
    if (forceTouch) {
      console.log('[skoffroad touch] force-touch mode active — keyboard events will fire on button press');
    }
  }

  // Run after DOM is ready
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }

})();
