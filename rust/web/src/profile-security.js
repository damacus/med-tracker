const pack = (value) =>
  btoa(String.fromCharCode.apply(null, new Uint8Array(value)))
    .replace(/\+/g, "-")
    .replace(/\//g, "_")
    .replace(/=/g, "");

const unpack = (value) =>
  Uint8Array.from(
    atob(value.replace(/-/g, "+").replace(/_/g, "/")),
    (character) => character.charCodeAt(0),
  );

const form = document.querySelector("[data-profile-passkey-register]");
if (form) {
  const trigger = form.querySelector("[data-passkey-create]");
  const output = form.querySelector('[name="webauthn_credential"]');
  const error = form.querySelector("[data-passkey-error]");
  trigger.addEventListener("click", async () => {
    error.textContent = "";
    if (!form.reportValidity()) return;
    if (!navigator.credentials || typeof navigator.credentials.create !== "function") {
      error.textContent = "This browser does not support passkeys.";
      return;
    }
    trigger.disabled = true;
    try {
      const options = JSON.parse(form.dataset.passkeyOptions);
      options.publicKey.challenge = unpack(options.publicKey.challenge);
      options.publicKey.user.id = unpack(options.publicKey.user.id);
      options.publicKey.excludeCredentials = (options.publicKey.excludeCredentials || []).map(
        (credential) => ({ ...credential, id: unpack(credential.id) }),
      );
      const credential = await navigator.credentials.create(options);
      if (!credential) return;
      output.value = JSON.stringify({
        id: pack(credential.rawId),
        rawId: pack(credential.rawId),
        type: credential.type,
        response: {
          attestationObject: pack(credential.response.attestationObject),
          clientDataJSON: pack(credential.response.clientDataJSON),
          transports: credential.response.getTransports?.() || [],
        },
      });
      form.requestSubmit();
    } catch (_failure) {
      error.textContent = "The passkey could not be created. Try again.";
    } finally {
      trigger.disabled = false;
    }
  });
}
