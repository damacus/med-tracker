async function refreshMobileShortcuts() {
  const match = location.pathname.match(/^\/households\/([^/]+)(?:\/|$)/);
  if (!match) return;
  try {
    const response = await fetch(`/households/${match[1]}/profile/navigation`, { headers: { Accept: 'application/json' }, cache: 'no-store' });
    if (!response.ok || !response.headers.get('content-type')?.includes('application/json')) return;
    const data = await response.json();
    if (!Array.isArray(data.links) || data.links.length > 3) return;
    const navigation = document.querySelector('[data-mobile-shortcuts]') || document.createElement('nav');
    navigation.dataset.mobileShortcuts = '';
    navigation.className = 'profile-mobile-shortcuts';
    navigation.setAttribute('aria-label', data.label);
    navigation.replaceChildren(...data.links.flatMap(link => {
      const url = new URL(link.href, location.origin);
      if (url.origin !== location.origin || !url.pathname.startsWith(`/households/${match[1]}/`)) return [];
      const anchor = document.createElement('a');
      anchor.href = url.pathname;
      anchor.setAttribute('aria-label', link.label);
      const icon = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
      icon.setAttribute('width', '24');
      icon.setAttribute('height', '24');
      icon.setAttribute('aria-hidden', 'true');
      const use = document.createElementNS('http://www.w3.org/2000/svg', 'use');
      if (!/^[a-z_]+$/.test(link.id)) return [];
      use.setAttribute('href', `/static/profile-icons.svg#${link.id}`);
      icon.append(use);
      const label = document.createElement('span');
      label.textContent = link.label;
      anchor.append(icon, label);
      if (location.pathname === url.pathname) anchor.setAttribute('aria-current', 'page');
      return [anchor];
    }));
    if (!navigation.isConnected) document.body.append(navigation);
    document.dispatchEvent(new CustomEvent('app-navigation', { detail: { ...data.shell, slug: match[1] } }));
  } catch { }
}
document.addEventListener('DOMContentLoaded', refreshMobileShortcuts);
document.addEventListener('profile-saved', refreshMobileShortcuts);
