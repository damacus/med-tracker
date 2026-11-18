(() => {
  const palettes = new Set(['default', 'serene-sage', 'modern-clinical', 'warm-earth', 'deep-lavender', 'forest-care', 'sunset-support', 'tech-indigo', 'soft-rose', 'minty-fresh', 'minimalist-monochrome']);
  const appearances = new Set(['light', 'dark', 'system']);
  const root = document.documentElement;
  const media = window.matchMedia('(prefers-color-scheme: dark)');
  function read(key, fallback) {
    try { return localStorage.getItem(key) || fallback; } catch { return fallback; }
  }
  function save(key, value) {
    try { localStorage.setItem(key, value); } catch {}
  }
  let palette = read('med-tracker-theme', 'default');
  let appearance = read('med-tracker-appearance', 'system');
  if (!palettes.has(palette)) palette = 'default';
  if (!appearances.has(appearance)) appearance = 'system';
  function household() {
    return document.querySelector('[data-profile-household]')?.dataset.profileHousehold
      || root.dataset.navigationHousehold || document.body?.dataset.navigationHousehold || '';
  }
  function paletteKey(slug) {
    return slug ? `med-tracker-theme-household:${slug}` : 'med-tracker-theme';
  }
  function selectedPalette() {
    const selected = read(paletteKey(household()), palette);
    return palettes.has(selected) ? selected : palette;
  }
  function apply() {
    const selected = selectedPalette();
    const effectiveAppearance = root.dataset.appearanceMode === 'system' ? 'system' : appearance;
    const dark = effectiveAppearance === 'dark' || (effectiveAppearance === 'system' && media.matches);
    root.dataset.appearance = effectiveAppearance;
    root.dataset.theme = `${root.dataset.allowPalette === 'true' ? selected : 'neutral'}-${dark ? 'dark' : 'light'}`;
    root.classList.toggle('dark', dark);
    const meta = document.querySelector('meta[name="theme-color"]');
    if (meta) meta.setAttribute('content', dark ? '#111827' : '#f8fafc');
    for (const illustration of document.querySelectorAll('[data-login-illustration]')) {
      const mode = dark ? 'dark' : 'light';
      illustration.querySelector('source').setAttribute('srcset', illustration.dataset[`${mode}Mobile`]);
      illustration.querySelector('img').setAttribute('src', illustration.dataset[`${mode}Desktop`]);
    }
    for (const button of document.querySelectorAll('[data-palette-choice]')) button.setAttribute('aria-pressed', String(button.dataset.paletteChoice === selected));
    for (const button of document.querySelectorAll('[data-appearance-choice]')) button.setAttribute('aria-pressed', String(button.dataset.appearanceChoice === appearance));
  }
  apply();
  media.addEventListener('change', apply);
  document.addEventListener('DOMContentLoaded', apply);
  document.addEventListener('click', event => {
    const paletteButton = event.target.closest('[data-palette-choice]');
    const appearanceButton = event.target.closest('[data-appearance-choice]');
    if (paletteButton && palettes.has(paletteButton.dataset.paletteChoice)) {
      const slug = household();
      const selected = paletteButton.dataset.paletteChoice;
      if (!slug) palette = selected;
      save(paletteKey(slug), selected);
    }
    if (appearanceButton && appearances.has(appearanceButton.dataset.appearanceChoice)) {
      appearance = appearanceButton.dataset.appearanceChoice;
      save('med-tracker-appearance', appearance);
    }
    if (paletteButton || appearanceButton) apply();
  });
})();
