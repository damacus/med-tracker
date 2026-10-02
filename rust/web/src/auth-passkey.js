const pack = (value) =>
  btoa(String.fromCharCode.apply(null, new Uint8Array(value)))
    .replace(/\+/g, "-")
    .replace(/\//g, "_")
    .replace(/=/g, "");

const unpack = (value) =>
  Uint8Array.from(
    atob(value.replace(/-/g, "+").replace(/_/g, "/")),
    (char) => char.charCodeAt(0),
  );

const initPasskeyLogin = () => {
  const form = document.getElementById("webauthn-login-form");
  const section = document.getElementById("passkey-login-section");
  const trigger = document.getElementById("passkey-login-trigger");
  const errorElement = document.getElementById("passkey-login-error");
  const authInput = document.getElementById("webauthn-auth");

  if (!form || !section || !trigger || !errorElement || !authInput) {
    return;
  }

  if (
    typeof window.PublicKeyCredential === "undefined" ||
    !navigator.credentials ||
    typeof navigator.credentials.get !== "function"
  ) {
    return;
  }

  section.hidden = false;

  const setError = (message) => {
    errorElement.textContent = message;
    errorElement.hidden = !message;
  };

  trigger.addEventListener("click", async () => {
    setError("");
    try {
      const options = JSON.parse(form.dataset.credentialOptions);
      options.challenge = unpack(options.challenge);
      options.allowCredentials = (options.allowCredentials || []).map(
        (credential) => ({ ...credential, id: unpack(credential.id) }),
      );
      const credential = await navigator.credentials.get({ publicKey: options });
      if (!credential) {
        return;
      }
      const authValue = {
        type: credential.type,
        id: pack(credential.rawId),
        rawId: pack(credential.rawId),
        response: {
          authenticatorData: pack(credential.response.authenticatorData),
          clientDataJSON: pack(credential.response.clientDataJSON),
          signature: pack(credential.response.signature),
        },
      };
      if (credential.response.userHandle) {
        authValue.response.userHandle = pack(credential.response.userHandle);
      }
      authInput.value = JSON.stringify(authValue);
      form.submit();
    } catch (error) {
      const cancelled =
        error && (error.name === "AbortError" || error.name === "NotAllowedError");
      setError(
        cancelled
          ? trigger.dataset.errorCancelled || trigger.dataset.errorFailed
          : trigger.dataset.errorFailed || trigger.dataset.errorUnsupported,
      );
    }
  });
};

if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", initPasskeyLogin, { once: true });
} else {
  initPasskeyLogin();
}
