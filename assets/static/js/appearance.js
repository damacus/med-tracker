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
  function apply() {
    const dark = appearance === 'dark' || (appearance === 'system' && media.matches);
    root.dataset.appearance = appearance;
    root.dataset.theme = `${root.dataset.allowPalette === 'true' ? palette : 'neutral'}-${dark ? 'dark' : 'light'}`;
    root.classList.toggle('dark', dark);
    const meta = document.querySelector('meta[name="theme-color"]');
    if (meta) meta.setAttribute('content', dark ? '#111827' : '#f8fafc');
    for (const button of document.querySelectorAll('[data-palette-choice]')) button.setAttribute('aria-pressed', String(button.dataset.paletteChoice === palette));
    for (const button of document.querySelectorAll('[data-appearance-choice]')) button.setAttribute('aria-pressed', String(button.dataset.appearanceChoice === appearance));
  }
  apply();
  media.addEventListener('change', apply);
  document.addEventListener('DOMContentLoaded', apply);
  document.addEventListener('click', event => {
    const paletteButton = event.target.closest('[data-palette-choice]');
    const appearanceButton = event.target.closest('[data-appearance-choice]');
    if (paletteButton && palettes.has(paletteButton.dataset.paletteChoice)) {
      palette = paletteButton.dataset.paletteChoice;
      save('med-tracker-theme', palette);
    }
    if (appearanceButton && appearances.has(appearanceButton.dataset.appearanceChoice)) {
      appearance = appearanceButton.dataset.appearanceChoice;
      save('med-tracker-appearance', appearance);
    }
    if (paletteButton || appearanceButton) apply();
  });
})();
