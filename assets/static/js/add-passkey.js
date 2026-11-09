export function mountAddPasskey(root = document) {
  const labels = JSON.parse(root.querySelector("[data-security-labels]")?.dataset.labels || "{}");
  const form = root.querySelector('[data-add-passkey]');
  if (!form || form.dataset.securityMounted === 'true') return;
  const eventRoot = form.closest('[data-profile-flow-root]') || document;
  form.addEventListener('submit', async event => {
    event.preventDefault();
    const button = form.querySelector('button');
    const error = root.querySelector('[data-registration-error]');
    button.disabled = true;
    eventRoot.dispatchEvent(new CustomEvent('security-flow-busy', { bubbles: true, detail: true }));
    error.hidden = true;
    let finalDispatched = false;
    try {
      const options = await fetch('/api/auth/passkey/generate-register-options', { credentials: 'same-origin', cache: 'no-store' });
      if (!options.ok) throw Error((labels.passkey_expired || "This confirmation has expired. Start the change again."));
      const credential = await navigator.credentials.create({ publicKey: PublicKeyCredential.parseCreationOptionsFromJSON(await options.json()) });
      if (!credential) throw Error((labels.passkey_cancelled || "Passkey registration was cancelled."));
      const body = JSON.stringify({ response: credential.toJSON(), name: new FormData(form).get('name') });
      finalDispatched = true;
      const result = await fetch('/api/auth/passkey/verify-registration', { method: 'POST', credentials: 'same-origin', headers: { 'Content-Type': 'application/json' }, body });
      if (result.status >= 400 && result.status < 500) finalDispatched = false;
      if (!result.ok) throw Error((labels.passkey_failed || "This passkey could not be registered. Start the change again."));
      await result.json();
      const navigation = new CustomEvent('security-flow-navigate', { bubbles: true, cancelable: true, detail: '/account/security' });
      if (eventRoot.dispatchEvent(navigation)) location.assign('/account/security');
    } catch (failure) {
      error.textContent = finalDispatched ? (labels.operation_uncertain || 'The request may have completed. Review current security settings before starting another change. A lost one-time key cannot be displayed again.') : failure.name === 'NotAllowedError' ? (labels.passkey_cancelled || "Passkey registration was cancelled.") : failure.message;
      if (finalDispatched) {
        const link = document.createElement('a');
        link.href = `${eventRoot.dataset?.flowProfile || '/profile'}#security`;
        link.textContent = labels.review_security || 'Review current security settings';
        link.className = 'link';
        link.dataset.profileFlowFallback = '';
        error.append(' ', link);
      }
      error.hidden = false;
      error.focus();
      button.disabled = finalDispatched;
    } finally {
      eventRoot.dispatchEvent(new CustomEvent('security-flow-busy', { bubbles: true, detail: false }));
    }
  });
  form.dataset.securityMounted = 'true';
  form.querySelector('button').disabled = false;
  root.querySelector('[data-passkey-script-required]')?.setAttribute('hidden', '');
}
if (typeof document !== 'undefined') mountAddPasskey(document);
