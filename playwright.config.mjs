import { defineConfig } from '@playwright/test';
import { readFileSync, readdirSync } from 'node:fs';
import { browserSelection } from './scripts/ci/loco-shards.mjs';

const selection = browserSelection(readdirSync(new URL('./tests/browser/', import.meta.url)).filter(file => file.endsWith('.spec.mjs')),
  process.env.LOCO_BROWSER_SHARD ?? '1/1', JSON.parse(readFileSync(new URL('./scripts/ci/browser-timings.json', import.meta.url), 'utf8')));
const matches = project => selection[project].length ? selection[project].map(file => `**/${file}`) : /$a/;

export default defineConfig({
  testDir: './tests/browser',
  testMatch: '**/*.spec.mjs',
  workers: process.env.CI ? 1 : 2,
  fullyParallel: false,
  timeout: 30000,
  reporter: [['line'], ['json', { outputFile: 'test-results/runtime-report.json' }]],
  outputDir: 'test-results/browser-output',
  use: { browserName: 'chromium', locale: 'en-GB', trace: 'retain-on-failure' },
  projects: [
    { name: 'desktop', testMatch: matches('desktop'), use: { viewport: { width: 1440, height: 1000 } } },
    { name: 'mobile', testMatch: matches('mobile'), use: { viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true } }
  ]
});
