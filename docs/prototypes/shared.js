'use strict';

{
const prototypePage = document.body.dataset.page;
const workspace = document.getElementById('main');
const themePreference = matchMedia('(prefers-color-scheme: dark)');
const themeKey = 'medtracker-prototype-theme';
if (new URLSearchParams(location.search).has('large-text')) document.documentElement.dataset.largeText = '';
let prototypeMode = 'light';
try {
  prototypeMode = sessionStorage.getItem(themeKey) || prototypeMode;
} catch {
  // The preview can also be opened directly from a local HTML file.
}

const destinations = [
  ['profile', 'Profile'],
  ['administration', 'Administration'],
];
const icons = {
  profile: '<circle cx="12" cy="8" r="3.5"/><path d="M5 21v-2a7 7 0 0 1 14 0v2"/>',
  administration: '<path d="M12 22s7-4 7-11V5l-7-3-7 3v6c0 7 7 11 7 11Z"/><path d="m9 11 2 2 4-4"/>',
};
const brand = '<a class="prototype-brand" href="index.html"><span aria-hidden="true">M</span>MedTracker</a>';
const navigation = `<nav class="shell-nav" aria-label="Prototype pages">${destinations.map(([page, label]) => (
  `<a href="${page}.html"${page === prototypePage ? ' aria-current="page"' : ''}><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" aria-hidden="true">${icons[page]}</svg>${label}</a>`
)).join('')}</nav>`;

if (workspace) {
  document.body.dataset.shell = '';
  workspace.insertAdjacentHTML('beforebegin', `
    <a class="skip-link" href="#main">Skip to content</a>
    <aside class="prototype-sidebar">${brand}${navigation}</aside>
    <header class="prototype-bar">
      ${brand}<span class="prototype-mode">Prototype · Sample data</span>
      <button type="button" id="prototype-theme"></button>${navigation}
    </header>`);
  workspace.tabIndex = -1;
}
const themeButton = document.getElementById('prototype-theme');

function updateTheme() {
  const dark = prototypeMode === 'dark' || (prototypeMode === 'system' && themePreference.matches);
  document.documentElement.dataset.theme = dark ? 'dark' : 'light';
  if (themeButton) themeButton.textContent = dark ? 'Light theme' : 'Dark theme';
}

window.prototypeTheme = (mode) => {
  if (!['light', 'dark', 'system'].includes(mode)) return;
  prototypeMode = mode;
  try {
    sessionStorage.setItem(themeKey, mode);
  } catch {
    // Applying the theme to the current preview does not require storage.
  }
  updateTheme();
};
themeButton?.addEventListener('click', () => {
  window.prototypeTheme(document.documentElement.dataset.theme === 'dark' ? 'light' : 'dark');
});
themePreference.addEventListener('change', updateTheme);
updateTheme();
}
