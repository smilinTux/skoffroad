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
        ...devices['iPhone 14'],
        // Override to Chromium engine (WebGPU support)
        channel: 'chromium',
      },
    },
  ],

  reporter: [
    ['list'],
    ['html', { open: 'never', outputFolder: 'playwright-report' }],
  ],
});
