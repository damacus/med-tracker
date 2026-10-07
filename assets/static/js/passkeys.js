function showPasskeyError(form) {
  let message = form.parentElement.querySelector('[role="alert"]');
  if (!message) {
    message = document.createElement('p');
    message.setAttribute('role', 'alert');
    message.className = 'shell-error';
    form.before(message);
  }
  message.textContent = 'Unable to use this passkey. Try again or use another sign-in method.';
}

document.querySelector('[data-passkey-login]')?.addEventListener('click', async event => {
  const button = event.currentTarget;
  const source = document.querySelector('form[action="/login"]');
  button.disabled = true;
  try {
    const optionsResponse = await fetch('/api/auth/passkey/generate-authenticate-options', { credentials: 'same-origin', cache: 'no-store' });
    if (!optionsResponse.ok) throw Error('Unavailable');
    const options = await optionsResponse.json();
    const publicKey = PublicKeyCredential.parseRequestOptionsFromJSON(options.publicKey);
    const credential = await navigator.credentials.get({ publicKey });
    const response = await fetch('/api/auth/passkey/verify-authentication', {
      method: 'POST', credentials: 'same-origin', cache: 'no-store',
      headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(credential.toJSON()),
    });
    if (!response.ok) throw Error('Unavailable');
    window.location.assign('/auth/passkey/complete');
  } catch {
    showPasskeyError(source);
    button.disabled = false;
  }
});
