# Passkeys

MedTracker supports passkey sign-in through Rodauth and WebAuthn. A passkey can
use a device unlock method, a password manager, or a compatible security key.

## Before you begin

Passkeys are bound to the site that created them. Production deployments must
set `APP_URL` to the public HTTPS origin, such as
`https://medtracker.example.com`. MedTracker uses this value for both the
WebAuthn origin and relying-party ID.

Changing the hostname or scheme after users register passkeys can stop those
passkeys from working. Plan a hostname change before registration starts.

WebAuthn is available on secure origins. Browsers also allow local development
on `localhost`.

## Add a passkey

1. Sign in with your existing method.
2. Open **Profile**.
3. Find **Two-Factor Authentication**, then **Passkeys**.
4. Select **Add a passkey**.
5. Confirm your password when MedTracker asks for it.
6. Follow the browser or device prompt.
7. Give the passkey a name that identifies the device or password manager.

MedTracker requires user verification during passkey registration and sign-in.
The authenticator must confirm the user with its supported unlock method.

Register more than one sign-in method before relying on a passkey. This reduces
the risk of losing access when a device is unavailable.

## Sign in with a passkey

Open the login page and select **Sign in with a passkey**. Choose the passkey in
the browser or device prompt and complete user verification.

Passkey autofill is also enabled. A supported browser can offer a MedTracker
passkey from the email field.

The browser controls whether a synced passkey or cross-device sign-in is
available. MedTracker does not copy private keys between devices.

## Remove a passkey

1. Sign in and open **Profile**.
2. Find the passkey under **Two-Factor Authentication**.
3. Select **Remove** beside that passkey.
4. Complete the fresh authentication check when prompted.

Removing a passkey does not remove other passkeys, an authenticator app, or
recovery codes.

## Recovery

Keep another sign-in method available. MedTracker supports authenticator-app
codes and recovery codes alongside passkeys.

Recovery codes are single-use secrets. Store them outside MedTracker and away
from the device that holds the passkey. Generating a new set invalidates the
old set.

If every sign-in method is unavailable, use the account recovery process or
contact the deployment administrator.

## Troubleshooting

### The browser does not offer a passkey

- Confirm that you opened the same HTTPS origin used when the passkey was
  registered.
- Check that the browser and device support WebAuthn.
- Try the explicit **Sign in with a passkey** action instead of autofill.

### The origin or relying-party ID does not match

Check `APP_URL`. It must contain the exact public scheme and host used by the
application. Restart the application after changing the value.

Do not weaken WebAuthn origin checks to work around a proxy or hostname error.
Correct the public URL and proxy configuration instead.

### A passkey device is lost

Use another registered passkey, an authenticator-app code, or a recovery code.
After signing in, remove the lost device's passkey and add a replacement.

## Current security settings

MedTracker configures passkeys as discoverable credentials and requires user
verification. The application stores each credential's public key and usage
counter. The authenticator keeps the private key.

The application does not require an authenticator vendor or request
direct attestation. Deployment administrators should not claim that MedTracker
has verified the hardware model behind a passkey.

## Related guides

- [Two-factor authentication](two-factor-authentication.md)
- [OpenID Connect setup](oidc-setup.md)
- [Self-hosting](self-hosting.md)
