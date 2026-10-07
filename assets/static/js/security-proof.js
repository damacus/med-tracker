(() => {
  const button = document.querySelector('[data-security-passkey]');
  if (!button) return;
  const error = document.querySelector('[data-proof-error]');
  async function send(path, data) {
    const response = await fetch(path, { method: 'POST', credentials: 'same-origin', cache: 'no-store', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(data) });
    if (!response.ok) throw new Error('Authentication failed. Retry this operation with your current passkey.');
    return response.json();
  }
  button.addEventListener('click', async () => {
    button.disabled = true;
    error.hidden = true;
    try {
      const operation_id = button.dataset.operation;
      const challenge = await send('/api/auth/security/passkey/start', { operation_id });
      const response = await navigator.credentials.get({ publicKey: PublicKeyCredential.parseRequestOptionsFromJSON(challenge.publicKey) });
      if (!response) throw new Error('Passkey confirmation was cancelled.');
      const result = await send('/api/auth/security/passkey/confirm', { operation_id, challenge_id: challenge.challenge_id, response: response.toJSON() });
      if (Array.isArray(result.recoveryCodes)) { document.dispatchEvent(new CustomEvent('recovery-generated', { detail: result })); return; }
      if (typeof result.apiKey === 'string') {
        const section = document.createElement('section');
        section.className = 'card shell-card max-w-xl mx-auto p-6 space-y-4';
        const heading = document.createElement('h1');
        heading.textContent = 'Save your API key';
        const text = document.createElement('p');
        text.textContent = 'Copy this key now. It will not be shown again.';
        const label = document.createElement('label');
        label.htmlFor = 'created-api-key';
        label.textContent = 'API key';
        const input = document.createElement('input');
        input.id = 'created-api-key';
        input.className = 'input input-bordered w-full';
        input.readOnly = true;
        input.value = result.apiKey;
        const link = document.createElement('a');
        link.href = '/account/security/keys';
        link.textContent = 'Return to personal API keys';
        section.append(heading, text, label, input, link);
        button.closest('section').replaceWith(section);
        input.focus();
        return;
      }
      location.assign(result.redirect || '/account/security');
    } catch (failure) {
      error.textContent = failure.name === 'NotAllowedError' ? 'Passkey confirmation was cancelled. Try again when ready.' : failure.message;
      error.hidden = false;
      error.focus();
      button.disabled = false;
    }
  });
})();
