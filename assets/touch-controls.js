/**
 * skoffroad – mobile touch controls
 * Sprint 62 — v0.25.0  (bugfix: Sprint 66 — Chromium touch emulation fixes)
 *
 * Strategy: synthesise KeyboardEvent dispatches so that Bevy's winit input
 * layer (which reads DOM keyboard events in WASM builds) sees exactly the
 * same events a physical keyboard would produce.
 *
 * ROOT CAUSE FIX (Sprint 62):
 *   winit 0.30 attaches its keydown/keyup listeners to the <canvas id="bevy">
 *   element directly (not to window or document).  DOM events dispatched on
 *   window bubble UP the chain — they can never reach a child element like the
 *   canvas.  Fix: dispatch synthetic KeyboardEvents on the canvas element
 *   first, then also on document and window so any additional listeners
 *   (splash-dismiss, bevy_matchbox, etc.) also see them.
 *
 *   We also call canvas.focus() before each dispatch so that the canvas is the
 *   active focus target, which is required for the browser to route real
 *   keyboard events to the element that winit is listening on.
 *
 * SPRINT 66 FIXES (Chromium iPhone 14 emulation regressions):
 *   1. setPointerCapture() is wrapped in try/catch — in Chromium touch
 *      emulation mode Playwright's synthesised pointer events can have a
 *      pointerId that Chromium considers "inactive" by the time the
 *      pointerdown handler runs, causing InvalidStateError that silently
 *      aborted keyDown() before it was called.
 *   2. KeyboardEvent constructor now includes keyCode / which (legacy fields)
 *      required by some Chromium / winit code paths that reject events without
 *      numeric key codes.
 *   3. Mobile menu overlay (#tc-menu-overlay) is built eagerly at init time
 *      instead of lazily on first showMenu() call.  Playwright's
 *      expect(overlay).not.toHaveClass() assertion on a non-existent element
 *      was hanging waiting for the element to appear in DOM.
 *
 * This keeps the Rust codebase completely unchanged — drive_input_keyboard
 * continues to work as-is.
 */

(function () {
  'use strict';

  /* ── Helpers ──────────────────────────────────────────────── */

  /**
   * Legacy keyCode / which values for each KeyboardEvent.code.
   * Required by winit and some Chromium code paths that reject synthetic events
   * that lack numeric key codes (Sprint 66 fix).
   */
  var KEY_CODE_MAP = {
    'KeyA': 65, 'KeyB': 66, 'KeyC': 67, 'KeyD': 68,
    'KeyE': 69, 'KeyF': 70, 'KeyG': 71, 'KeyH': 72,
    'KeyI': 73, 'KeyJ': 74, 'KeyK': 75, 'KeyL': 76,
    'KeyM': 77, 'KeyN': 78, 'KeyO': 79, 'KeyP': 80,
    'KeyQ': 81, 'KeyR': 82, 'KeyS': 83, 'KeyT': 84,
    'KeyU': 85, 'KeyV': 86, 'KeyW': 87, 'KeyX': 88,
    'KeyY': 89, 'KeyZ': 90,
    'Space': 32, 'Tab': 9, 'Escape': 27, 'Enter': 13,
    'Backspace': 8, 'Delete': 46,
    'ArrowLeft': 37, 'ArrowUp': 38, 'ArrowRight': 39, 'ArrowDown': 40,
    'ShiftLeft': 16, 'ShiftRight': 16,
    'ControlLeft': 17, 'ControlRight': 17,
    'AltLeft': 18, 'AltRight': 18,
    'Digit0': 48, 'Digit1': 49, 'Digit2': 50, 'Digit3': 51,
    'Digit4': 52, 'Digit5': 53, 'Digit6': 54, 'Digit7': 55,
    'Digit8': 56, 'Digit9': 57,
  };

  /**
   * Dispatch a synthetic KeyboardEvent on the canvas (#bevy), document, and
   * window.  Dispatching on the canvas is required for Bevy/winit 0.30 because
   * it attaches its keydown/keyup listeners directly to the canvas element.
   * Dispatching on document and window catches any additional listeners
   * (splash-dismiss, bevy_matchbox, etc.).
   *
   * We call canvas.focus() first so that the browser routes subsequent REAL
   * keyboard events from physical hardware to the correct element too.
   *
   * opts may include extra KeyboardEventInit fields (e.g. shiftKey).
   */
  function getCanvas() {
    return document.getElementById('bevy');
  }

  function fireKey(type, code, key, extraOpts) {
    var kc = KEY_CODE_MAP[code] || 0;
    var opts = {
      code: code,
      key: key || code,
      keyCode: kc,
      which: kc,
      bubbles: true,
      cancelable: true,
    };
    // Merge any extra options (e.g. shiftKey: true for Shift+Tab).
    if (extraOpts) {
      for (var k in extraOpts) {
        if (Object.prototype.hasOwnProperty.call(extraOpts, k)) {
          opts[k] = extraOpts[k];
        }
      }
    }

    var canvas = getCanvas();
    if (canvas) {
      // Ensure canvas has focus so real keyboard events also reach winit.
      // Use { preventScroll: true } to avoid jarring scroll-to-canvas on mobile.
      try { canvas.focus({ preventScroll: true }); } catch (e) {}
      canvas.dispatchEvent(new KeyboardEvent(type, opts));
    }

    // Also dispatch on document and window for any window-level listeners
    // (splash dismiss, bevy_matchbox channel relay, etc.).
    document.dispatchEvent(new KeyboardEvent(type, opts));
    window.dispatchEvent(new KeyboardEvent(type, opts));
  }

  function keyDown(code, key) { fireKey('keydown', code, key); }
  function keyUp(code, key)   { fireKey('keyup',   code, key); }

  /* ── Touch detection ──────────────────────────────────────── */

  var params       = new URLSearchParams(window.location.search);
  var forceTouch   = params.has('force-touch');
  var isTouchDevice =
    forceTouch ||
    'ontouchstart' in window ||
    navigator.maxTouchPoints > 0;

  /* ── Mobile menu overlay ──────────────────────────────────── */

  /**
   * Build the fullscreen mobile menu overlay.
   * Each row dispatches the matching desktop hotkey to open that panel.
   */
  function buildMobileMenu() {
    var overlay = document.createElement('div');
    overlay.id = 'tc-menu-overlay';
    overlay.setAttribute('aria-modal', 'true');
    overlay.setAttribute('role', 'dialog');
    overlay.setAttribute('aria-label', 'Mobile menu');

    var header = document.createElement('div');
    header.id = 'tc-menu-header';

    var title = document.createElement('span');
    title.textContent = 'MENU';
    header.appendChild(title);

    var closeBtn = document.createElement('button');
    closeBtn.id = 'tc-menu-close';
    closeBtn.type = 'button';
    closeBtn.setAttribute('aria-label', 'Close menu');
    closeBtn.textContent = '✕';
    closeBtn.addEventListener('pointerdown', function (e) {
      e.preventDefault();
      hideMenu();
    });
    header.appendChild(closeBtn);
    overlay.appendChild(header);

    var list = document.createElement('ul');
    list.id = 'tc-menu-list';

    /**
     * Menu entries: [ icon, label, KeyCode, key ]
     * Tapping a row dispatches the keydown event to enter that desktop panel.
     */
    var entries = [
      ['🔧', 'Vehicle Mods (M)',         'ShiftLeft+KeyM', null, 'ShiftM'],
      ['🌐', 'Multiplayer (I)',           'KeyI',           'i',  null],
      ['🏆', 'Hillclimb Leaderboard (H)', 'KeyH',           'h',  null],
      ['🗺️', 'Custom Map (drag-drop)',    'KeyM',           'm',  null],
      ['🎤', 'Voice / Webcam (Q)',        'KeyQ',           'q',  null],
      ['🎯', 'Mission Select (Shift+Tab)','Tab',            'Tab', 'ShiftTab'],
      ['❓', 'Help (Esc)',               'Escape',         'Escape', null],
    ];

    entries.forEach(function (entry) {
      var icon    = entry[0];
      var label   = entry[1];
      var code    = entry[2];
      var keyVal  = entry[3];
      var special = entry[4];

      var li = document.createElement('li');
      li.className = 'tc-menu-item';

      var iconEl = document.createElement('span');
      iconEl.className = 'tc-menu-icon';
      iconEl.textContent = icon;

      var labelEl = document.createElement('span');
      labelEl.className = 'tc-menu-label';
      labelEl.textContent = label;

      li.appendChild(iconEl);
      li.appendChild(labelEl);

      li.addEventListener('pointerdown', function (e) {
        e.preventDefault();
        hideMenu();

        // ShiftM / ShiftTab need a two-event sequence (Shift down, key down, key up, Shift up).
        // Uses the top-level fireKey() helper so keyCode/which are populated correctly.
        function fireShiftKey(mainCode, mainKey) {
          fireKey('keydown', 'ShiftLeft', 'Shift', { shiftKey: true });
          fireKey('keydown', mainCode,    mainKey,  { shiftKey: true });
          fireKey('keyup',   mainCode,    mainKey,  { shiftKey: true });
          fireKey('keyup',   'ShiftLeft', 'Shift',  { shiftKey: false });
        }
        if (special === 'ShiftM') {
          fireShiftKey('KeyM', 'M');
        } else if (special === 'ShiftTab') {
          fireShiftKey('Tab', 'Tab');
        } else {
          keyDown(code, keyVal || code);
          setTimeout(function () { keyUp(code, keyVal || code); }, 80);
        }
      });

      list.appendChild(li);
    });

    overlay.appendChild(list);

    // Dismiss on backdrop tap (tapping outside the panel).
    overlay.addEventListener('pointerdown', function (e) {
      if (e.target === overlay) {
        e.preventDefault();
        hideMenu();
      }
    });

    document.body.appendChild(overlay);
    return overlay;
  }

  // _menuOverlay is set to the DOM element during init() — see initMenu().
  // It is NOT lazily built because the Playwright test asserts
  // `expect('#tc-menu-overlay').not.toHaveClass(/tc-menu-open/)` BEFORE
  // tapping the MENU button; a lazy build would leave the element absent from
  // DOM and cause the assertion to hang (Sprint 66 fix).
  var _menuOverlay = null;

  function initMenu() {
    _menuOverlay = buildMobileMenu();
  }

  function showMenu() {
    if (_menuOverlay) _menuOverlay.classList.add('tc-menu-open');
  }

  function hideMenu() {
    if (_menuOverlay) _menuOverlay.classList.remove('tc-menu-open');
  }

  /* ── Build DOM ────────────────────────────────────────────── */

  /**
   * Build the full touch-controls overlay and inject it into <body>.
   * Returns the root element so the toggle can show/hide it.
   */
  function buildOverlay() {
    // ── Root ────────────────────────────────────────────────
    var root = document.createElement('div');
    root.id = 'touch-controls';

    // ── Joystick ────────────────────────────────────────────
    var stickZone = document.createElement('div');
    stickZone.id = 'tc-stick-zone';

    var stickRing = document.createElement('div');
    stickRing.id = 'tc-stick-ring';

    var stickThumb = document.createElement('div');
    stickThumb.id = 'tc-stick-thumb';

    stickRing.appendChild(stickThumb);
    stickZone.appendChild(stickRing);
    root.appendChild(stickZone);

    // ── Button grid ─────────────────────────────────────────
    var buttons = document.createElement('div');
    buttons.id = 'tc-buttons';

    /**
     * @param {string} id       element id
     * @param {string} icon     emoji / unicode icon
     * @param {string} label    short text label
     * @param {string} code     KeyboardEvent.code to synthesise
     * @param {string} [key]    KeyboardEvent.key (defaults to code)
     */
    function makeBtn(id, icon, label, code, key) {
      var btn = document.createElement('button');
      btn.id = id;
      btn.className = 'tc-btn';
      btn.setAttribute('aria-label', label);
      btn.type = 'button';

      var iconEl = document.createElement('span');
      iconEl.className = 'tc-btn-icon';
      iconEl.textContent = icon;

      var labelEl = document.createElement('span');
      labelEl.textContent = label;

      btn.appendChild(iconEl);
      btn.appendChild(labelEl);

      // All buttons: keydown on pointerdown, keyup on release/cancel/leave.
      btn.addEventListener('pointerdown', function (e) {
        e.preventDefault();
        // setPointerCapture can throw InvalidStateError in Chromium touch
        // emulation when the synthesised pointerId is not "active" at the time
        // the handler runs (Sprint 66 fix: guard with try/catch so keyDown()
        // is always called even if capture fails).
        try { btn.setPointerCapture(e.pointerId); } catch (_) {}
        btn.classList.add('tc-pressed');
        keyDown(code, key || code);
      });
      var releaseBtn = function (e) {
        e.preventDefault();
        btn.classList.remove('tc-pressed');
        keyUp(code, key || code);
      };
      btn.addEventListener('pointerup',     releaseBtn);
      btn.addEventListener('pointercancel', releaseBtn);

      return btn;
    }

    // Row 1: FWD | Camera | Menu
    buttons.appendChild(makeBtn('tc-btn-fwd',  '▲', 'FWD',  'KeyW', 'w'));
    buttons.appendChild(makeBtn('tc-btn-cam',  '📷', 'V',   'KeyV', 'v'));

    var menuBtn = document.createElement('button');
    menuBtn.id = 'tc-btn-menu';
    menuBtn.className = 'tc-btn';
    menuBtn.setAttribute('aria-label', 'Open mobile menu');
    menuBtn.type = 'button';
    var menuIconEl = document.createElement('span');
    menuIconEl.className = 'tc-btn-icon';
    menuIconEl.textContent = '☰';
    var menuLabelEl = document.createElement('span');
    menuLabelEl.textContent = 'MENU';
    menuBtn.appendChild(menuIconEl);
    menuBtn.appendChild(menuLabelEl);
    menuBtn.addEventListener('pointerdown', function (e) {
      e.preventDefault();
      menuBtn.classList.add('tc-pressed');
      showMenu();
    });
    menuBtn.addEventListener('pointerup',     function (e) { e.preventDefault(); menuBtn.classList.remove('tc-pressed'); });
    menuBtn.addEventListener('pointercancel', function (e) { e.preventDefault(); menuBtn.classList.remove('tc-pressed'); });
    buttons.appendChild(menuBtn);

    // Row 2: REV | PTT | Horn
    buttons.appendChild(makeBtn('tc-btn-rev',  '▼', 'REV',  'KeyS', 's'));
    buttons.appendChild(makeBtn('tc-btn-ptt',  '🎙️', 'F',   'KeyF', 'f'));
    buttons.appendChild(makeBtn('tc-btn-horn', '📯', 'HORN','KeyN', 'n'));

    // Row 3: Reset | Photo | Esc
    buttons.appendChild(makeBtn('tc-btn-reset', '🔄', 'R',   'KeyR',   'r'));
    buttons.appendChild(makeBtn('tc-btn-photo', '🖼️', 'P',  'KeyP',   'p'));
    buttons.appendChild(makeBtn('tc-btn-esc',   '⏸', 'Esc', 'Escape', 'Escape'));

    // Row 4: Brake (full-width — hold to brake)
    var brakeBtn = makeBtn('tc-btn-brake', '🛑', 'BRAKE', 'Space', ' ');
    buttons.appendChild(brakeBtn);

    root.appendChild(buttons);

    document.body.appendChild(root);
    return root;
  }

  /* ── Desktop toggle pill ──────────────────────────────────── */

  function buildToggle(overlayRoot) {
    var pill = document.createElement('button');
    pill.id = 'tc-toggle';
    pill.type = 'button';
    pill.textContent = 'touch HUD';
    document.body.appendChild(pill);

    var visible = isTouchDevice;
    if (!visible) overlayRoot.classList.add('tc-hidden');

    pill.addEventListener('click', function () {
      visible = !visible;
      overlayRoot.classList.toggle('tc-hidden', !visible);
      pill.textContent = visible ? 'hide touch HUD' : 'touch HUD';
    });
  }

  /* ── Joystick logic ───────────────────────────────────────── */

  /** Keys currently held down by the joystick (to avoid redundant events). */
  var stickHeld = { KeyW: false, KeyS: false, KeyA: false, KeyD: false };

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
   * Dead-zone threshold: 0.20 (reduced from 0.25 — wider usable range on mobile)
   */
  var DEAD = 0.20;

  function applyStick(nx, ny) {
    setStickKey('KeyW', 'w',  ny >  DEAD);
    setStickKey('KeyS', 's', -ny >  DEAD);
    setStickKey('KeyA', 'a', -nx >  DEAD);
    setStickKey('KeyD', 'd',  nx >  DEAD);
  }

  function initJoystick(overlayRoot) {
    var zone  = overlayRoot.querySelector('#tc-stick-zone');
    var thumb = overlayRoot.querySelector('#tc-stick-thumb');

    // Outer ring radius (px) — half of the zone's 140 px width minus thumb half
    var RING_R  = 70;   // px — outer ring radius
    var THUMB_R = 30;   // px — thumb radius (half of 60 px)
    var MAX_R   = RING_R - THUMB_R;  // max thumb travel from centre

    var activeTouchId = null;
    var originX = 0;
    var originY = 0;

    function getZoneCenter() {
      var rect = zone.getBoundingClientRect();
      return { cx: rect.left + rect.width / 2, cy: rect.top + rect.height / 2 };
    }

    function updateThumb(nx, ny) {
      // nx, ny in [-1, 1]
      var dx = nx * MAX_R;
      var dy = -ny * MAX_R; // screen Y is inverted
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
      // Guard with try/catch for the same reason as button setPointerCapture
      // (Sprint 66 fix: Chromium touch emulation may throw InvalidStateError).
      try { zone.setPointerCapture(e.pointerId); } catch (_) {}
      activeTouchId = e.pointerId;
      var center = getZoneCenter();
      originX = center.cx;
      originY = center.cy;
    });

    zone.addEventListener('pointermove', function (e) {
      if (e.pointerId !== activeTouchId) return;
      e.preventDefault();

      // Screen-space delta from zone centre (screen Y increases downward).
      var sdx = e.clientX - originX;
      var sdy = e.clientY - originY;
      var dist = Math.sqrt(sdx * sdx + sdy * sdy);
      var clamped = Math.min(dist, MAX_R);
      var angle = Math.atan2(sdy, sdx); // atan2 in screen space

      // Normalised screen-space components [-1, 1].
      var nsx = clamped === 0 ? 0 : (Math.cos(angle) * clamped) / MAX_R;
      var nsy = clamped === 0 ? 0 : (Math.sin(angle) * clamped) / MAX_R; // +ve = down on screen

      // Logical-space Y: invert so +ve = up (forward drive).
      var nx = nsx;
      var ny = -nsy;

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
    var overlayRoot = buildOverlay();
    buildToggle(overlayRoot);
    initJoystick(overlayRoot);

    // Build the mobile menu overlay eagerly so #tc-menu-overlay is present in
    // the DOM from the start (Sprint 66: fixes Playwright assertion on absent element).
    initMenu();

    if (isTouchDevice) {
      // Prevent the browser's default scroll/zoom on the canvas so touch
      // events reach our handlers cleanly.
      var canvas = document.getElementById('bevy');
      if (canvas) {
        canvas.style.touchAction = 'none';
        // Ensure canvas is focusable — winit sets tabindex="0" but may not
        // call focus() itself until the first pointer event on the canvas.
        if (!canvas.getAttribute('tabindex')) {
          canvas.setAttribute('tabindex', '0');
        }
        // Focus the canvas once on init so winit's keyboard listeners are
        // active immediately (user may not have tapped the canvas yet).
        try { canvas.focus({ preventScroll: true }); } catch (e) {}
      }
    }

    // Console hint for dev verification
    if (forceTouch) {
      console.log('[skoffroad touch] force-touch mode active — keyboard events dispatched to canvas + document + window');
    }
  }

  // Run after DOM is ready
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }

})();
