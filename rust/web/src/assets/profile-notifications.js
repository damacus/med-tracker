const pushPanel = document.querySelector("[data-profile-push]");

if (pushPanel) {
  const status = pushPanel.querySelector("[data-push-status]");
  const on = pushPanel.querySelector("[data-push-on]");
  const off = pushPanel.querySelector("[data-push-off]");
  const test = pushPanel.querySelector("[data-push-test]");
  const csrf = pushPanel.dataset.csrf;
  const householdId = pushPanel.dataset.householdId;
  const key = document.querySelector('meta[name="vapid-public-key"]')?.content;
  const path = `/api/v1/households/${encodeURIComponent(householdId)}/push_subscription`;
  const supported = "Notification" in window && "serviceWorker" in navigator && "PushManager" in window;

  const message = value => { status.textContent = value; };
  const buttons = (enabled, disabled = false) => {
    on.disabled = disabled || enabled;
    off.disabled = disabled || !enabled;
    on.setAttribute("aria-pressed", String(enabled));
    off.setAttribute("aria-pressed", String(!enabled));
    test.hidden = !enabled;
  };
  const registration = async () => {
    await navigator.serviceWorker.register("/sw.js", { scope: "/" });
    return navigator.serviceWorker.ready;
  };
  const decodeKey = value => {
    const normalized = value.replace(/-/g, "+").replace(/_/g, "/");
    const padded = normalized.padEnd(Math.ceil(normalized.length / 4) * 4, "=");
    return Uint8Array.from(atob(padded), character => character.charCodeAt(0));
  };
  const refresh = async () => {
    if (!supported) { buttons(false, true); message("Push notifications are not supported in this browser."); return; }
    if (!key) { buttons(false, true); message("Push notifications are not configured."); return; }
    if (Notification.permission === "denied") { buttons(false, true); message("Notifications are blocked in your browser settings."); return; }
    const subscription = await (await registration()).pushManager.getSubscription();
    const enabled = Notification.permission === "granted" && !!subscription;
    buttons(enabled);
    message(enabled ? "Notifications are fully enabled." : "Notifications are not enabled on this device.");
  };
  const send = async (method, body) => {
    const url = method === "DELETE" ? `${path}?endpoint=${encodeURIComponent(body.push_subscription.endpoint)}` : path;
    const response = await fetch(url, {
      method, credentials: "same-origin",
      headers: { "Content-Type": "application/json", "X-CSRF-Token": csrf },
      body: method === "POST" ? JSON.stringify(body) : undefined
    });
    if (!response.ok) throw new Error(`Subscription request failed (${response.status}).`);
  };
  on.addEventListener("click", async () => {
    on.disabled = true;
    try {
      if (await Notification.requestPermission() !== "granted") { await refresh(); return; }
      const service = await registration();
      const subscription = await service.pushManager.subscribe({ userVisibleOnly: true, applicationServerKey: decodeKey(key) });
      const data = subscription.toJSON();
      await send("POST", { push_subscription: { endpoint: data.endpoint, keys: data.keys } });
      await refresh();
    } catch (failure) { message(failure.message); on.disabled = false; }
  });
  off.addEventListener("click", async () => {
    off.disabled = true;
    try {
      const subscription = await (await registration()).pushManager.getSubscription();
      if (subscription) {
        await send("DELETE", { push_subscription: { endpoint: subscription.endpoint } });
        await subscription.unsubscribe();
      }
      await refresh();
    } catch (failure) { message(failure.message); off.disabled = false; }
  });
  test.addEventListener("click", async () => {
    test.disabled = true;
    try {
      const response = await fetch(`${path}/test`, {
        method: "POST", credentials: "same-origin", headers: { "X-CSRF-Token": csrf }
      });
      if (!response.ok) throw new Error(`Test notification failed (${response.status}).`);
      message("Test notification requested from server.");
    } catch (failure) { message(failure.message); }
    finally { test.disabled = false; }
  });
  refresh().catch(failure => { buttons(false, true); message(failure.message); });
}
