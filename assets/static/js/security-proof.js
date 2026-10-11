(() => {
  const labels = JSON.parse(document.querySelector("[data-security-labels]")?.dataset.labels || "{}");
  const button = document.querySelector('[data-security-passkey]');
  if (!button) return;
  const error = document.querySelector('[data-proof-error]');
  async function send(path, data) {
    const response = await fetch(path, { method: 'POST', credentials: 'same-origin', cache: 'no-store', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(data) });
    if (!response.ok) throw new Error((labels.passkey_proof_failed || "Authentication failed. Retry this operation with your current passkey."));
    return response.json();
  }
  button.addEventListener('click', async () => {
    button.disabled = true;
    document.dispatchEvent(new CustomEvent('security-flow-busy', { detail: true }));
    error.hidden = true;
    try {
      const operation_id = button.dataset.operation;
      const challenge = await send('/api/auth/security/passkey/start', { operation_id });
      const response = await navigator.credentials.get({ publicKey: PublicKeyCredential.parseRequestOptionsFromJSON(challenge.publicKey) });
      if (!response) throw new Error((labels.passkey_proof_cancelled || "Passkey confirmation was cancelled."));
      const result = await send('/api/auth/security/passkey/confirm', { operation_id, challenge_id: challenge.challenge_id, response: response.toJSON() });
      if (Array.isArray(result.recoveryCodes)) { document.dispatchEvent(new CustomEvent('recovery-generated', { detail: result })); return; }
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
        return;
      }
      const destination = result.redirect || '/account/security';
      const navigation = new CustomEvent('security-flow-navigate', { cancelable: true, detail: destination });
      if (document.dispatchEvent(navigation)) location.assign(destination);
    } catch (failure) {
      error.textContent = failure.name === 'NotAllowedError' ? (labels.passkey_retry || "Passkey confirmation was cancelled. Try again when ready.") : failure.message;
      error.hidden = false;
      error.focus();
      button.disabled = false;
    } finally {
      document.dispatchEvent(new CustomEvent('security-flow-busy', { detail: false }));
    }
  });
})();
