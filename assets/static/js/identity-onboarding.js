(() => {
  const passwordField = document.querySelector('[data-signup-password]');
  if (passwordField) {
    const updateCredential = () => {
      const password = document.querySelector('input[name="credential"]:checked').value === 'password';
      passwordField.hidden = !password;
      passwordField.querySelector('input').disabled = !password;
    };
    updateCredential();
    for (const choice of document.querySelectorAll('input[name="credential"]')) choice.addEventListener('change', updateCredential);
  }
  const passkey = document.querySelector('[data-passkey-enrolment]');
  const complete = document.querySelector('[data-complete-onboarding]');
  if (!passkey && !complete) return;
  const error = document.querySelector('[data-onboarding-error]');
  const status = document.querySelector('[data-onboarding-status]');
  function failed(failure) {
    status.textContent = '';
    error.textContent = failure.name === 'NotAllowedError' ? 'Passkey registration was cancelled. Try again when you are ready.' : failure.message || 'Your sign-in method could not be saved. Try again.';
    error.hidden = false;
    error.focus();
  }
  async function json(response) {
    if (!response.ok) throw new Error('Your sign-in method could not be saved. Check your details and try again.');
    return response.json();
  }
  async function finish() {
    await json(await fetch('/api/auth/onboarding/complete', { method: 'POST', credentials: 'same-origin', cache: 'no-store', headers: { 'Content-Type': 'application/json' }, body: '{}' }));
    location.assign('/');
  }
  complete?.addEventListener('click', async () => {
    complete.disabled = true;
    try { await finish(); } catch (failure) { failed(failure); complete.disabled = false; }
  });
  passkey?.addEventListener('submit', async event => {
    event.preventDefault();
    const button = passkey.querySelector('button');
    button.disabled = true;
    error.hidden = true;
    status.textContent = 'Follow your device’s instructions to create a passkey.';
    try {
      if (!window.PublicKeyCredential?.parseCreationOptionsFromJSON) throw new Error('This browser cannot register a passkey. Use a password or open your verification email in a browser that supports passkeys.');
      const options = await json(await fetch('/api/auth/passkey/generate-register-options', { credentials: 'same-origin', cache: 'no-store' }));
      const credential = await navigator.credentials.create({ publicKey: PublicKeyCredential.parseCreationOptionsFromJSON(options) });
      if (!credential) throw new Error('Passkey registration was cancelled. Try again when you are ready.');
      await json(await fetch('/api/auth/passkey/verify-registration', { method: 'POST', credentials: 'same-origin', cache: 'no-store', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ response: credential.toJSON(), name: new FormData(passkey).get('name') }) }));
      await finish();
    } catch (failure) { failed(failure); button.disabled = false; }
  });
})();
