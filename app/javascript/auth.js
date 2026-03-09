// Lightweight auth entrypoint.
// Intentionally avoids loading Turbo + Stimulus controller graph.

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

const noopError = (error) =>
  error?.name === "AbortError" || error?.name === "NotAllowedError";

const initPasskeyLogin = () => {
  const form = document.getElementById("webauthn-login-form");
  const section = document.getElementById("passkey-login-section");
  const trigger = document.getElementById("passkey-login-trigger");
  const errorElement = document.getElementById("passkey-login-error");
  const authInput = document.getElementById("webauthn-auth");

  if (!form || !section || !trigger || !errorElement || !authInput) {
    return false;
  }

  if (form.dataset.passkeyInitialized === "true") {
    return true;
  }

  if (
    typeof window.PublicKeyCredential === "undefined" ||
    !navigator.credentials ||
    typeof navigator.credentials.get !== "function"
  ) {
    trigger.hidden = true;
    trigger.disabled = true;
    return false;
  }

  section.hidden = false;
  trigger.hidden = false;
  trigger.disabled = false;

  let conditionalController = null;

  const setError = (message) => {
    if (message) {
      errorElement.textContent = message;
      errorElement.hidden = false;
    } else {
      errorElement.textContent = "";
      errorElement.hidden = true;
    }
  };

  const credentialOptions = () => {
    const options = JSON.parse(form.dataset.credentialOptions);
    options.challenge = unpack(options.challenge);
    options.allowCredentials = (options.allowCredentials || []).map((credential) => ({
      ...credential,
      id: unpack(credential.id),
    }));
    return options;
  };

  const submitCredential = (credential) => {
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
  };

  const requestCredential = async ({ mediation = undefined, signal = undefined } = {}) => {
    const options = { publicKey: credentialOptions() };

    if (mediation) {
      options.mediation = mediation;
    }

    if (signal) {
      options.signal = signal;
    }

    const credential = await navigator.credentials.get(options);

    if (credential) {
      submitCredential(credential);
    }
  };

  const startConditionalAutofill = async () => {
    if (typeof PublicKeyCredential.isConditionalMediationAvailable !== "function") {
      return;
    }

    const conditionalAvailable = await PublicKeyCredential.isConditionalMediationAvailable();

    if (!conditionalAvailable) {
      return;
    }

    conditionalController =
      typeof AbortController === "function" ? new AbortController() : null;

    try {
      await requestCredential({
        mediation: "conditional",
        signal: conditionalController?.signal,
      });
    } catch (error) {
      if (!noopError(error)) {
        setError(trigger.dataset.errorFailed);
      }
    }
  };

  trigger.addEventListener("click", async () => {
    setError("");

    if (conditionalController) {
      conditionalController.abort();
      conditionalController = null;
    }

    try {
      await requestCredential();
    } catch (error) {
      if (!noopError(error)) {
        setError(trigger.dataset.errorFailed || trigger.dataset.errorUnsupported);
      }
    }
  });

  startConditionalAutofill().catch(() => {
    setError(trigger.dataset.errorFailed);
  });

  form.dataset.passkeyInitialized = "true";
  return true;
};

const bootPasskeyLogin = () => {
  initPasskeyLogin();
};

window.MedTrackerAuth = window.MedTrackerAuth || {};
window.MedTrackerAuth.initPasskeyLogin = bootPasskeyLogin;

if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", bootPasskeyLogin, { once: true });
} else {
  bootPasskeyLogin();
}
