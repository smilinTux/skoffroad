import { defineConfig, devices } from '@playwright/test';

export default defineConfig({
  testDir: '.',
  testMatch: '**/*.spec.ts',
  timeout: 90_000,
  retries: 0,
  workers: 1,

  use: {
    baseURL: 'http://localhost:8080',
    // iPhone 14 profile: viewport 390x844, touch events, mobile UA
    ...devices['iPhone 14'],
    // Allow slow WASM load
    navigationTimeout: 60_000,
    actionTimeout: 30_000,
  },

  projects: [
    {
      name: 'iPhone 14 (Chromium)',
      use: {
        // iPhone 14 viewport + touch settings, but run under Chromium engine.
        // WebKit on Linux CI has frequent launch issues; Chromium is reliable.
        // We spread only the viewport/UA/touch properties from the device
        // descriptor and override browserName so Playwright uses Chromium.
        viewport: devices['iPhone 14'].viewport,
        userAgent: devices['iPhone 14'].userAgent,
        deviceScaleFactor: devices['iPhone 14'].deviceScaleFactor,
        isMobile: devices['iPhone 14'].isMobile,
        hasTouch: devices['iPhone 14'].hasTouch,
        browserName: 'chromium',
      },
    },
  ],

  reporter: [
    ['list'],
    ['html', { open: 'never', outputFolder: 'playwright-report' }],
  ],
});
