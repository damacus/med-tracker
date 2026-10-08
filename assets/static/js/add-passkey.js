(() => {
  const form = document.querySelector('[data-add-passkey]');
  if (!form) return;
  form.addEventListener('submit', async event => {
    event.preventDefault();
    const button = form.querySelector('button');
    const error = document.querySelector('[data-registration-error]');
    button.disabled = true;
    error.hidden = true;
    try {
      const operation = encodeURIComponent(form.dataset.operation);
      const options = await fetch(`/api/auth/passkey/generate-register-options?operation_id=${operation}`, { credentials: 'same-origin', cache: 'no-store' });
      if (!options.ok) throw Error('This confirmation has expired. Start the change again.');
      const credential = await navigator.credentials.create({ publicKey: PublicKeyCredential.parseCreationOptionsFromJSON(await options.json()) });
      const result = await fetch('/api/auth/passkey/verify-registration', { method: 'POST', credentials: 'same-origin', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ response: credential.toJSON(), name: new FormData(form).get('name') }) });
      if (!result.ok) throw Error('This passkey could not be registered. Start the change again.');
      location.assign('/account/security');
    } catch (failure) {
      error.textContent = failure.name === 'NotAllowedError' ? 'Passkey registration was cancelled.' : failure.message;
      error.hidden = false;
      error.focus();
      button.disabled = false;
    }
  });
})();
