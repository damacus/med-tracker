import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './tests/browser',
  testMatch: '**/*.spec.mjs',
  workers: process.env.CI ? 1 : 2,
  fullyParallel: false,
  timeout: 30000,
  reporter: [['line'], ['json', { outputFile: 'test-results/runtime-report.json' }]],
  outputDir: 'test-results',
  use: { browserName: 'chromium', locale: 'en-GB', trace: 'retain-on-failure' },
  projects: [
    { name: 'desktop', use: { viewport: { width: 1440, height: 1000 } } },
    { name: 'mobile', use: { viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true } }
  ]
});
