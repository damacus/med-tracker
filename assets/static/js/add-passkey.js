(() => {
  const labels = JSON.parse(document.querySelector("[data-security-labels]")?.dataset.labels || "{}");
  const form = document.querySelector('[data-add-passkey]');
  if (!form) return;
  form.addEventListener('submit', async event => {
    event.preventDefault();
    const button = form.querySelector('button');
    const error = document.querySelector('[data-registration-error]');
    button.disabled = true;
    document.dispatchEvent(new CustomEvent('security-flow-busy', { detail: true }));
    error.hidden = true;
    try {
      const operation = encodeURIComponent(form.dataset.operation);
      const options = await fetch(`/api/auth/passkey/generate-register-options?operation_id=${operation}`, { credentials: 'same-origin', cache: 'no-store' });
      if (!options.ok) throw Error((labels.passkey_expired || "This confirmation has expired. Start the change again."));
      const credential = await navigator.credentials.create({ publicKey: PublicKeyCredential.parseCreationOptionsFromJSON(await options.json()) });
      const result = await fetch('/api/auth/passkey/verify-registration', { method: 'POST', credentials: 'same-origin', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ response: credential.toJSON(), name: new FormData(form).get('name') }) });
      if (!result.ok) throw Error((labels.passkey_failed || "This passkey could not be registered. Start the change again."));
      const navigation = new CustomEvent('security-flow-navigate', { cancelable: true, detail: '/account/security' });
      if (document.dispatchEvent(navigation)) location.assign('/account/security');
    } catch (failure) {
      error.textContent = failure.name === 'NotAllowedError' ? (labels.passkey_cancelled || "Passkey registration was cancelled.") : failure.message;
      error.hidden = false;
      error.focus();
      button.disabled = false;
    } finally {
      document.dispatchEvent(new CustomEvent('security-flow-busy', { detail: false }));
    }
  });
})();
