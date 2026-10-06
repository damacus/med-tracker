import { test, expect } from './fixtures.mjs';
import { readFileSync } from 'node:fs';

const palettes = ['default', 'serene-sage', 'modern-clinical', 'warm-earth', 'deep-lavender', 'forest-care', 'sunset-support', 'tech-indigo', 'soft-rose', 'minty-fresh'];

test('public shell preserves signed-out appearance and a saved palette before first paint', async ({ page }) => {
  await page.emulateMedia({ colorScheme: 'dark' });
  await page.addInitScript(() => {
    localStorage.setItem('med-tracker-theme', 'warm-earth');
    localStorage.setItem('med-tracker-appearance', 'system');
  });
  await page.goto('/');
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'neutral-dark');
  await expect(page.getByRole('heading', { name: 'MedTracker', exact: true })).toBeVisible();
  expect(await page.evaluate(() => localStorage.getItem('med-tracker-theme'))).toBe('warm-earth');
  await page.emulateMedia({ colorScheme: 'light' });
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'neutral-light');
  expect(await page.evaluate(() => document.querySelector('script[src$="appearance.js"]').compareDocumentPosition(document.querySelector('link[rel="stylesheet"]')) & Node.DOCUMENT_POSITION_FOLLOWING)).toBeTruthy();
});

test('all existing palette exports retain light and dark colour formulas and font families', async ({ page }) => {
  await page.goto('/');
  const reference = readFileSync('rails/app/assets/tailwind/application.css', 'utf8');
  const tokenRules = [...reference.replace(/\/\*[\s\S]*?\*\//g, '').matchAll(/([^{}]+)\{([^{}]*)\}/g)]
    .filter(match => /^\s*(?::root|\.dark)/.test(match[1]))
    .map(match => `${match[1]} {${match[2]}}`).join('\n');
  await page.evaluate(css => {
    const frame = document.createElement('iframe');
    frame.id = 'theme-reference';
    frame.hidden = true;
    document.body.append(frame);
    frame.contentDocument.head.innerHTML = `<style>${css}</style>`;
  }, tokenRules);
  for (const palette of [...palettes, 'minimalist-monochrome']) {
    for (const appearance of ['light', 'dark']) {
      const tokens = await page.evaluate(({ palette, appearance }) => {
        const sample = document.createElement('div');
        sample.dataset.theme = `${palette}-${appearance}`;
        document.body.append(sample);
        const style = getComputedStyle(sample);
        const names = ['--primary', '--color-primary', '--background', '--border-color', '--border', '--font-family'];
        const result = Object.fromEntries(names.map(token => [token, style.getPropertyValue(token).trim()]));
        const canvas = document.createElement('canvas');
        canvas.width = canvas.height = 1;
        const context = canvas.getContext('2d');
        const pixel = value => {
          context.clearRect(0, 0, 1, 1);
          context.fillStyle = value;
          context.fillRect(0, 0, 1, 1);
          return [...context.getImageData(0, 0, 1, 1).data];
        };
        const luminance = value => {
          const linear = pixel(value).slice(0, 3).map(channel => {
            channel /= 255;
            return channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4;
          });
          return linear[0] * 0.2126 + linear[1] * 0.7152 + linear[2] * 0.0722;
        };
        sample.style.color = 'var(--foreground)';
        sample.style.backgroundColor = 'var(--card)';
        const visible = getComputedStyle(sample);
        const foreground = luminance(visible.color);
        const background = luminance(visible.backgroundColor);
        result.textContrast = (Math.max(foreground, background) + 0.05) / (Math.min(foreground, background) + 0.05);
        const referenceRoot = document.getElementById('theme-reference').contentDocument.documentElement;
        referenceRoot.dataset.allowPalette = 'true';
        referenceRoot.className = `${appearance === 'dark' ? 'dark' : ''} ${palette !== 'default' ? `theme-${palette}` : ''}`;
        const referenceStyle = referenceRoot.ownerDocument.defaultView.getComputedStyle(referenceRoot);
        result.reference = Object.fromEntries(['--primary', '--background', '--font-family'].map(token => [token, referenceStyle.getPropertyValue(token).trim()]));
        sample.style.color = 'var(--primary)';
        sample.style.backgroundColor = 'var(--background)';
        sample.style.fontFamily = 'var(--font-family)';
        referenceRoot.style.color = 'var(--primary)';
        referenceRoot.style.backgroundColor = 'var(--background)';
        referenceRoot.style.fontFamily = 'var(--font-family)';
        result.resolved = { primary: pixel(getComputedStyle(sample).color), background: pixel(getComputedStyle(sample).backgroundColor), font: getComputedStyle(sample).fontFamily };
        result.referenceResolved = { primary: pixel(referenceStyle.color), background: pixel(referenceStyle.backgroundColor), font: referenceStyle.fontFamily };
        sample.remove();
        return result;
      }, { palette, appearance });
      expect(tokens['--primary']).not.toBe('');
      expect(tokens['--color-primary']).toBe(tokens['--primary']);
      expect(tokens['--background']).not.toBe('');
      expect(tokens['--border']).toBe('1px');
      expect(tokens['--border-color']).not.toBe('1px');
      expect(tokens['--font-family']).not.toBe('');
      expect(tokens.resolved).toEqual(tokens.referenceResolved);
      expect(tokens.textContrast).toBeGreaterThanOrEqual(4.5);
    }
  }
});

test('appearance controls persist choices while public surfaces stay neutral', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Appearance', exact: true }).click();
  await page.getByRole('button', { name: 'Dark', exact: true }).click();
  await page.getByRole('button', { name: 'Warm Earth', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Warm Earth', exact: true })).toHaveAttribute('aria-pressed', 'true');
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'neutral-dark');
  await page.keyboard.press('Escape');
  await page.reload();
  expect(await page.evaluate(() => localStorage.getItem('med-tracker-theme'))).toBe('warm-earth');
  expect(await page.evaluate(() => localStorage.getItem('med-tracker-appearance'))).toBe('dark');
});

test('appearance dialog supports Escape and returns focus', async ({ page }) => {
  await page.goto('/');
  const appearance = page.getByRole('button', { name: 'Appearance', exact: true });
  await appearance.click();
  await expect(page.getByRole('dialog', { name: 'Appearance' })).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(appearance).toBeFocused();
});

test('desktop and mobile shell load local assets without overflow', async ({ page, request }, testInfo) => {
  await page.goto('/');
  await page.evaluate(() => document.fonts.ready);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  const font = await request.get('/static/fonts/plus-jakarta-sans/plus-jakarta-sans-v12-latin-regular.woff2');
  expect(font.ok()).toBe(true);
  const geist = await request.get('/static/fonts/geist/geist-v1.7.2-variable.woff2');
  expect(geist.ok()).toBe(true);
  for (const family of ['Inter', 'Plus Jakarta Sans', 'Lexend', 'Outfit', 'Figtree', 'Urbanist', 'Public Sans', 'Geist']) {
    expect(await page.evaluate(async family => (await document.fonts.load(`400 16px "${family}"`)).some(face => face.family.replaceAll('"', '') === family && face.status === 'loaded'), family)).toBe(true);
  }
  const missing = await request.get('/static/not-a-file.js');
  expect(missing.headers()['content-type']).toContain('text/html');
  expect(await missing.text()).toContain('Asset not found');
  await page.screenshot({ path: `docs/screenshots/loco-shell-${testInfo.project.name}.png`, fullPage: true });
});
