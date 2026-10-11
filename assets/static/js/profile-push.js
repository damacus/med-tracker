(() => {
  const section = document.querySelector('[data-profile-push]');
  if (!section) return;
  const status = section.querySelector('[data-push-status]');
  const enable = section.querySelector('[data-push-enable]');
  const disable = section.querySelector('[data-push-disable]');
  const test = section.querySelector('[data-push-test]');
  let configuration;
  let subscription;
  let subscribed = false;
  let pending = false;
  function render(label) {
    status.textContent = section.dataset[label];
    enable.disabled = pending || !configuration?.configured || Notification.permission === 'denied' || subscribed;
    disable.disabled = pending || !subscription;
    test.hidden = !subscribed;
    test.disabled = pending;
  }
  async function send(path, body, method = 'POST') {
    const response = await fetch(`${section.dataset.endpoint}${path}`, { method, credentials: 'same-origin', cache: 'no-store', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ authenticity_token: section.dataset.token, ...body }) });
    if ([401, 403, 404].includes(response.status)) { location.reload(); throw Error('Access changed'); }
    if (!response.ok) throw Error('Notification request failed');
    return response.json();
  }
  async function perform(action) {
    if (pending) return;
    pending = true;
    render(subscribed ? 'enabled' : 'disabled');
    try { await action(); }
    catch { status.textContent = section.dataset.failed; }
    finally {
      pending = false;
      const message = status.textContent;
      render(subscribed ? 'enabled' : 'disabled');
      status.textContent = message;
    }
  }
  enable.addEventListener('click', () => perform(async () => {
    if (await Notification.requestPermission() !== 'granted') { render('denied'); return; }
    await navigator.serviceWorker.register('/profile-service-worker.js', { scope: '/' });
    const worker = await navigator.serviceWorker.ready;
    subscription = await worker.pushManager.getSubscription();
    const existing = Boolean(subscription);
    if (!subscription) {
      const encoded = configuration.public_key.replace(/-/g, '+').replace(/_/g, '/');
      const key = Uint8Array.from(atob(encoded.padEnd(Math.ceil(encoded.length / 4) * 4, '=')), value => value.charCodeAt(0));
      subscription = await worker.pushManager.subscribe({ userVisibleOnly: true, applicationServerKey: key });
    }
    try { await send('', { subscription: subscription.toJSON() }); }
    catch (failure) {
      if (!existing) { await subscription.unsubscribe(); subscription = null; }
      throw failure;
    }
    subscribed = true;
    render('enabled');
  }));
  disable.addEventListener('click', () => perform(async () => {
    if (!subscription) return;
    await send('', { endpoint: subscription.endpoint }, 'DELETE');
    subscribed = false;
    if (!await subscription.unsubscribe()) throw Error('Browser unsubscribe failed');
    subscription = null;
    render('disabled');
  }));
  test.addEventListener('click', () => perform(async () => {
    if (!subscription || !subscribed) return;
    const result = await send('/test', { endpoint: subscription.endpoint });
    if (result.status === 'expired') {
      subscribed = false;
      if (!await subscription.unsubscribe()) throw Error('Expired subscription could not be removed');
      subscription = null;
      render('expired');
    }
    else render(result.status === 'accepted' ? 'accepted' : 'failed');
  }));
  async function check() {
    try {
      const response = await fetch(section.dataset.endpoint, { credentials: 'same-origin', cache: 'no-store' });
      if (!response.ok) throw Error('Notification status unavailable');
      configuration = await response.json();
      if (!configuration.configured) { status.textContent = section.dataset.unconfigured; return; }
      if (!('serviceWorker' in navigator) || !('PushManager' in window) || !('Notification' in window)) { status.textContent = section.dataset.unsupported; return; }
      const worker = await navigator.serviceWorker.getRegistration('/');
      subscription = await worker?.pushManager.getSubscription();
      if (subscription) subscribed = (await send('/status', { endpoint: subscription.endpoint })).subscribed;
      render(Notification.permission === 'denied' ? 'denied' : subscribed ? 'enabled' : 'disabled');
    } catch { status.textContent = section.dataset.failed; }
  }
  void check();
})();
