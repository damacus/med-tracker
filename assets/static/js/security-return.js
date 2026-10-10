(() => {
  const key = 'medtracker.profile.security.return';
  try {
    if (location.pathname === '/login') { sessionStorage.removeItem(key); return; }
    if (location.pathname !== '/account/security') return;
    const saved = JSON.parse(sessionStorage.getItem(key) || 'null');
    sessionStorage.removeItem(key);
    if (!saved || typeof saved.path !== 'string' || !Number.isFinite(saved.expires) || saved.expires <= Date.now()) return;
    if (!/^\/households\/[a-zA-Z0-9_-]+\/profile#security$/.test(saved.path)) return;
    location.replace(saved.path);
  } catch {}
})();
