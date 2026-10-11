(() => {
  const passwordField = document.querySelector('[data-signup-password]');
  if (passwordField) {
    const updateCredential = () => {
      const password = document.querySelector('input[name="credential"]:checked').value === 'password';
      passwordField.hidden = !password;
      passwordField.querySelector('input').disabled = !password;
    };
    updateCredential();
    for (const choice of document.querySelectorAll('input[name="credential"]')) {
      choice.addEventListener('change', updateCredential);
    }
  }
  const form = document.querySelector('[data-passkey-enrolment]');
  const showRecovery = document.querySelector('[data-show-recovery]');
  if (!form && !showRecovery) return;
  const error = document.querySelector('[data-onboarding-error]');
  const status = document.querySelector('[data-onboarding-status]');
  const list = document.querySelector('[data-recovery-list]');
  const recovery = document.querySelector('[data-recovery-codes]');
  const submit = form?.querySelector('button[type="submit"]');
  let codes = [];
  let generation;

  function showError(message) {
    status.textContent = '';
    error.textContent = message;
    error.hidden = false;
    error.focus();
  }

  async function jsonResponse(response) {
    if (!response.ok) throw new Error('Your passkey could not be registered. Try again. If this page has expired, open your verification email again.');
    return response.json();
  }

  async function revealRecovery() {
    const result = await jsonResponse(await fetch('/api/auth/onboarding/recovery-codes', {
      method: 'POST', credentials: 'same-origin', cache: 'no-store',
      headers: { 'Content-Type': 'application/json' }, body: '{}',
    }));
    if (!Array.isArray(result.recoveryCodes) || result.recoveryCodes.length !== 10 || result.recoveryCodes.some(code => typeof code !== 'string')) {
      throw new Error('Your recovery codes could not be displayed. Keep this page open and contact support before continuing.');
    }
    codes = result.recoveryCodes;
    generation = result.generation;
    for (const code of codes) {
      const item = document.createElement('li');
      item.textContent = code;
      list.append(item);
    }
    document.querySelector('[data-enrolment-form]').hidden = true;
    status.textContent = '';
    recovery.hidden = false;
    document.querySelector('[data-recovery-heading]').focus();
  }

  showRecovery?.addEventListener('click', async () => {
    showRecovery.disabled = true;
    try { await revealRecovery(); }
    catch (failure) {
      showError(failure.message);
      showRecovery.disabled = false;
    }
  });

  form?.addEventListener('submit', async event => {
    event.preventDefault();
    if (submit.disabled) return;
    error.hidden = true;
    if (!window.PublicKeyCredential?.parseCreationOptionsFromJSON) {
      showError('This browser cannot register a passkey. Open your verification email in a browser that supports passkeys.');
      return;
    }
    submit.disabled = true;
    status.textContent = 'Follow your device’s instructions to create a passkey.';
    try {
      const name = new FormData(form).get('name');
      const options = await jsonResponse(await fetch('/api/auth/passkey/generate-register-options', {
        credentials: 'same-origin', cache: 'no-store',
      }));
      const credential = await navigator.credentials.create({ publicKey: PublicKeyCredential.parseCreationOptionsFromJSON(options) });
      if (!credential) throw new Error('Passkey registration was cancelled. Try again when you are ready.');
      await jsonResponse(await fetch('/api/auth/passkey/verify-registration', {
        method: 'POST', credentials: 'same-origin', cache: 'no-store',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ response: credential.toJSON(), name }),
      }));
      await revealRecovery();
    } catch (failure) {
      showError(failure.name === 'NotAllowedError' ? 'Passkey registration was cancelled. Try again when you are ready.' : failure.message || 'Your passkey could not be registered. Try again.');
      submit.disabled = false;
    }
  });

  document.querySelector('[data-recovery-copy]').addEventListener('click', async () => {
    try {
      await navigator.clipboard.writeText(codes.join('\n'));
      status.textContent = 'Recovery codes copied. Keep them somewhere safe.';
    } catch {
      showError('The codes could not be copied. Use Download codes or write them down.');
    }
  });

  document.querySelector('[data-recovery-download]').addEventListener('click', () => {
    const url = URL.createObjectURL(new Blob([`MedTracker recovery codes\n\n${codes.join('\n')}\n`], { type: 'text/plain' }));
    const link = document.createElement('a');
    link.href = url;
    link.download = 'medtracker-recovery-codes.txt';
    link.click();
    URL.revokeObjectURL(url);
    status.textContent = 'Recovery codes downloaded. Keep the file somewhere safe.';
  });

  function clearCodes() {
    codes = [];
    list.replaceChildren();
  }
  document.querySelector('[data-recovery-continue]').addEventListener('click', async () => {
    if (!document.querySelector('[data-recovery-saved]').checked) {
      showError('Save your recovery codes before continuing.');
      return;
    }
    try {
      await jsonResponse(await fetch('/api/auth/onboarding/complete', {
        method: 'POST', credentials: 'same-origin', cache: 'no-store',
        headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ saved: true, generation }),
      }));
      clearCodes();
      window.location.assign('/');
    } catch (failure) { showError(failure.message); }
  });
  window.addEventListener('pagehide', clearCodes);
})();
