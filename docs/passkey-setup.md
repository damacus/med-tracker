# Passkeys

MedTracker uses Better Auth's native passkey plugin. A passkey can use a device
unlock method, a password manager or a compatible security key. Loco starts with
fresh authentication data; passkeys registered in Rails are not imported.

## Before you begin

Passkeys are bound to the site that created them. Configure Loco's public server
origin, including `MEDTRACKER_PUBLIC_HOST` in production, to match the URL people
actually use. Production requires HTTPS. The authentication plugin derives its
WebAuthn origin and relying-party host from that configured origin.

Changing the hostname or scheme after registration can stop existing passkeys
from working. Plan hostname changes before registration starts. Browsers permit
local development on `localhost`.

## Add a passkey

1. Sign in and open **Account security**.
2. Select **Add passkey**.
3. Follow the browser or device prompt.
4. Give the passkey a name that identifies the device or password manager.

Initial account setup can also enrol a passkey after email verification.
MedTracker requires user verification during registration and sign-in, and
requests a discoverable credential. Complete the authenticator's device-unlock
or user-verification prompt.

Keep another sign-in method available before relying on a passkey. This reduces
the risk of losing access when a device is unavailable.

## Sign in with a passkey

Choose passkey sign-in on the login page. Select the passkey in the browser or
device prompt and complete user verification. Browser and password-manager
support determines whether autofill, a synced passkey or cross-device sign-in is
available. MedTracker does not copy private keys between devices.

## Remove a passkey

Open **Account security** and select **Remove passkey** beside the credential.
Removing a passkey does not remove other credentials or change household
permissions.

## Recovery

Use another configured sign-in method if a passkey is unavailable. Native MFA
backup codes complete a second-factor challenge after the first sign-in step;
they cannot sign in alone. Email password recovery does not remove configured
MFA. See the [authentication and recovery guide](two-factor-authentication.md).

Keep backup codes outside MedTracker and away from the authenticator device. If
every configured method is unavailable, contact the deployment administrator.

## Troubleshooting

### The browser does not offer a passkey

- Open the same HTTPS origin used when the passkey was registered.
- Check that the browser and device support WebAuthn.
- Try explicit passkey sign-in instead of autofill.
- Complete the device-unlock or user-verification prompt.

### The origin or relying-party ID does not match

Check Loco's configured public server origin and production
`MEDTRACKER_PUBLIC_HOST`. The browser URL and configured scheme, host and public
port must agree. Restart the application after changing its configuration.

### A device or password manager was lost

Sign in with another configured method, remove the unavailable passkey, then add
its replacement. Do not change the public hostname as a recovery shortcut.
