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

const setup = document.querySelector('#webauthn-setup-form');
setup?.addEventListener('submit', async event => {
  event.preventDefault();
  const button = setup.querySelector('button[type="submit"]');
  button.disabled = true;
  try {
    const options = JSON.parse(setup.dataset.credentialOptions);
    const publicKey = PublicKeyCredential.parseCreationOptionsFromJSON(options.publicKey);
    const credential = await navigator.credentials.create({ publicKey });
    setup.elements.webauthn_setup.value = JSON.stringify(credential.toJSON());
    HTMLFormElement.prototype.submit.call(setup);
  } catch {
    showPasskeyError(setup);
    button.disabled = false;
  }
});

document.querySelector('[data-passkey-login]')?.addEventListener('click', async event => {
  const button = event.currentTarget;
  const source = document.querySelector('[data-passkey-form], form[action="/login"]');
  button.disabled = true;
  try {
    let options;
    if (source.dataset.credentialOptions) {
      options = JSON.parse(source.dataset.credentialOptions);
    } else {
      const response = await fetch('/api/auth/passkey/generate-authenticate-options', { credentials: 'same-origin', cache: 'no-store' });
      if (!response.ok) throw Error('Unavailable');
      options = await response.json();
    }
    const publicKey = PublicKeyCredential.parseRequestOptionsFromJSON(options.publicKey);
    const credential = await navigator.credentials.get({ publicKey });
    if (!button.dataset.passkeyAction) {
      const response = await fetch('/api/auth/passkey/verify-authentication', {
        method: 'POST', credentials: 'same-origin', cache: 'no-store',
        headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(credential.toJSON()),
      });
      if (!response.ok) throw Error('Unavailable');
      window.location.assign('/');
      return;
    }
    const form = document.createElement('form');
    form.method = 'post';
    form.action = button.dataset.passkeyAction || '/webauthn-login';
    for (const [name, value] of Object.entries({ authenticity_token: source.elements.authenticity_token.value, webauthn_auth: JSON.stringify(credential.toJSON()) })) {
      const input = document.createElement('input');
      input.type = 'hidden';
      input.name = name;
      input.value = value;
      form.append(input);
    }
    document.body.append(form);
    HTMLFormElement.prototype.submit.call(form);
  } catch {
    showPasskeyError(source);
    button.disabled = false;
  }
});
