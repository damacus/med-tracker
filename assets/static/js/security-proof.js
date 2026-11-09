export function mountSecurityProof(root = document) {
  const labels = JSON.parse(root.querySelector("[data-security-labels]")?.dataset.labels || "{}");
  const button = root.querySelector('[data-security-passkey]');
  if (!button || button.dataset.securityMounted === 'true') return;
  const eventRoot = button.closest('[data-profile-flow-root]') || document;
  const error = root.querySelector('[data-proof-error]');
  async function send(path, data) {
    const response = await fetch(path, { method: 'POST', credentials: 'same-origin', cache: 'no-store', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(data) });
    if (!response.ok) throw new Error((labels.passkey_proof_failed || "Authentication failed. Retry this operation with your current passkey."));
    return response.json();
  }
  button.addEventListener('click', async () => {
    button.disabled = true;
    eventRoot.dispatchEvent(new CustomEvent('security-flow-busy', { bubbles: true, detail: true }));
    error.hidden = true;
    let finalDispatched = false;
    try {
      const operation_id = button.dataset.operation;
      const challenge = await send('/api/auth/security/passkey/start', { operation_id });
      const publicKey = PublicKeyCredential.parseRequestOptionsFromJSON(challenge);
      publicKey.userVerification = 'required';
      const response = await navigator.credentials.get({ publicKey });
      if (!response) throw new Error((labels.passkey_proof_cancelled || "Passkey confirmation was cancelled."));
      const body = JSON.stringify({ operation_id, response: response.toJSON() });
      finalDispatched = true;
      const confirmation = await fetch('/api/auth/security/passkey/confirm', { method: 'POST', credentials: 'same-origin', cache: 'no-store', headers: { 'Content-Type': 'application/json' }, body });
      if (confirmation.status >= 400 && confirmation.status < 500) finalDispatched = false;
      if (!confirmation.ok) throw Error(labels.passkey_proof_failed || 'Authentication failed. Retry this operation with your current passkey.');
      const result = await confirmation.json();
      if (!result || typeof result !== 'object' || typeof result.apiKey !== 'string' && result.status !== true) throw Error('Unexpected confirmation response');
      if (typeof result.apiKey === 'string') {
        const section = document.createElement('section');
        section.className = 'card shell-card max-w-xl mx-auto p-6 space-y-4';
        const heading = document.createElement('h1');
        heading.textContent = (labels.save_key || "Save your API key");
        const text = document.createElement('p');
        text.textContent = (labels.key_copy_short || "Copy this key now. It will not be shown again.");
        const label = document.createElement('label');
        label.htmlFor = 'created-api-key';
        label.textContent = (labels.api_key || "API key");
        const input = document.createElement('input');
        input.id = 'created-api-key';
        input.className = 'input input-bordered w-full';
        input.readOnly = true;
        input.value = result.apiKey;
        const link = document.createElement('a');
        link.href = '/account/security/keys';
        link.textContent = (labels.return_keys || "Return to personal API keys");
        section.append(heading, text, label, input, link);
        button.closest('section').replaceWith(section);
        input.focus();
        if (eventRoot !== document) {
          eventRoot.dataset.flowOutcome = 'secret-result';
          eventRoot.dataset.flowSecret = 'true';
          eventRoot.dispatchEvent(new CustomEvent('security-flow-result', { bubbles: true }));
        }
        return;
      }
      const destination = result.redirect || '/account/security';
      const navigation = new CustomEvent('security-flow-navigate', { bubbles: true, cancelable: true, detail: destination });
      if (eventRoot.dispatchEvent(navigation)) location.assign(destination);
    } catch (failure) {
      error.textContent = finalDispatched ? (labels.operation_uncertain || 'The request may have completed. Review current security settings before starting another change. A lost one-time key cannot be displayed again.') : failure.name === 'NotAllowedError' ? (labels.passkey_retry || "Passkey confirmation was cancelled. Try again when ready.") : failure.message;
      if (finalDispatched) {
        const scope = eventRoot === document ? button.closest('section') : eventRoot;
        scope.querySelectorAll('button[type="submit"], input[type="submit"], [data-security-passkey]').forEach(control => { control.disabled = true; });
        scope.querySelectorAll('input[type="password"]').forEach(input => { input.value = ''; input.removeAttribute('value'); });
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
  button.dataset.securityMounted = 'true';
}
if (typeof document !== 'undefined') mountSecurityProof(document);
