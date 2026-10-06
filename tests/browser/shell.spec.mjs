import { test, expect } from './fixtures.mjs';

const palettes = ['default', 'serene-sage', 'modern-clinical', 'warm-earth', 'deep-lavender', 'forest-care', 'sunset-support', 'tech-indigo', 'soft-rose', 'minty-fresh'];

test('public shell preserves first-paint appearance, accessible themes, local assets and saved controls', async ({ page, request }, testInfo) => {
  await test.step('saved palette stays neutral before first paint and follows system appearance', async () => {
    await page.emulateMedia({ colorScheme: 'dark' });
    await page.addInitScript(() => {
      if (localStorage.getItem('med-tracker-theme') === null) localStorage.setItem('med-tracker-theme', 'warm-earth');
      if (localStorage.getItem('med-tracker-appearance') === null) localStorage.setItem('med-tracker-appearance', 'system');
    });
    await page.goto('/');
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'neutral-dark');
    await expect(page.getByRole('heading', { name: 'MedTracker', exact: true })).toBeVisible();
    expect(await page.evaluate(() => localStorage.getItem('med-tracker-theme'))).toBe('warm-earth');
    await page.emulateMedia({ colorScheme: 'light' });
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'neutral-light');
    expect(await page.evaluate(() => document.querySelector('script[src$="appearance.js"]').compareDocumentPosition(document.querySelector('link[rel="stylesheet"]')) & Node.DOCUMENT_POSITION_FOLLOWING)).toBeTruthy();
  });

  await test.step('every colour scheme has light and dark tokens with accessible text contrast', async () => {
    for (const palette of [...palettes, 'minimalist-monochrome']) {
      for (const appearance of ['light', 'dark']) {
        const tokens = await page.evaluate(({ palette, appearance }) => {
          const sample = document.createElement('div');
          sample.dataset.theme = `${palette}-${appearance}`;
          document.body.append(sample);
          const style = getComputedStyle(sample);
          const names = ['--primary', '--color-primary', '--background', '--border-color', '--font-family'];
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
          sample.remove();
          return result;
        }, { palette, appearance });
        for (const token of ['--primary', '--color-primary', '--background', '--border-color', '--font-family']) {
          expect(tokens[token], `${palette}-${appearance} ${token}`).not.toBe('');
        }
        expect(tokens.textContrast, `${palette}-${appearance} text contrast`).toBeGreaterThanOrEqual(4.5);
      }
    }
  });

  await test.step('desktop and mobile assets load locally without horizontal overflow', async () => {
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
    await page.screenshot({ path: testInfo.outputPath(`loco-shell-${testInfo.project.name}.png`), fullPage: true });
  });

  await test.step('appearance dialog returns focus and saves user choices across reload', async () => {
    const appearance = page.getByRole('button', { name: 'Appearance', exact: true });
    await appearance.click();
    await expect(page.getByRole('dialog', { name: 'Appearance' })).toBeVisible();
    await page.getByRole('button', { name: 'Dark', exact: true }).click();
    await page.getByRole('button', { name: 'Command Centre', exact: true }).click();
    await page.getByRole('button', { name: 'Warm Earth', exact: true }).click();
    await expect(page.getByRole('button', { name: 'Warm Earth', exact: true })).toHaveAttribute('aria-pressed', 'true');
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'neutral-dark');
    await page.keyboard.press('Escape');
    await expect(page.getByRole('dialog', { name: 'Appearance' })).not.toBeVisible();
    await expect(appearance).toBeFocused();
    await page.reload();
    expect(await page.evaluate(() => localStorage.getItem('med-tracker-theme'))).toBe('warm-earth');
    expect(await page.evaluate(() => localStorage.getItem('med-tracker-appearance'))).toBe('dark');
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'neutral-dark');
  });

});
