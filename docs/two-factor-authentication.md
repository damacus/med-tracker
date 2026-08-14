# Two-Factor Authentication

MedTracker supports authenticator-app codes, passkeys, and recovery codes.
Household owners and administrators must configure a second factor.

## Choose your methods

Use at least two independent ways to sign in:

- An **authenticator app** creates a six-digit time-based code.
- A **passkey** uses a device, password manager, or security key.
- A **recovery code** is a single-use backup for when another method is
  unavailable.

Recovery codes are not a primary method. MedTracker lets you generate them only
after you configure an authenticator app or passkey.

## Set up an authenticator app

1. Sign in and open **Profile**.
2. Under **Two-Factor Authentication**, select **Set up authenticator app**.
3. Scan the QR code with a compatible authenticator app.
4. Enter the current six-digit code to confirm setup.
5. Generate recovery codes and store them safely.

The code changes every 30 seconds. MedTracker labels the account as
**MedTracker** in the authenticator app.

To replace a lost authenticator, sign in with another method, disable the old
configuration, and set it up again.

## Set up a passkey

Open **Profile**, then select **Add a passkey** under **Two-Factor
Authentication**. Follow the browser prompt and give the passkey a name that
identifies where it is stored.

MedTracker requires user verification for passkeys. See the [passkey
guide](passkey-setup.md) for deployment requirements and troubleshooting.

## Generate recovery codes

1. Configure an authenticator app or passkey.
2. Open **Profile** and select **Generate recovery codes**.
3. Complete fresh authentication when prompted.
4. Save every code outside MedTracker.

Each code works once. Generating a replacement set invalidates all old codes.
Treat the codes like passwords and keep them away from the device used for
your other sign-in method.

MedTracker stores the recovery-code value needed for verification. Do not
describe the database column as encrypted unless the storage design changes.

## Sign in

After password sign-in, MedTracker asks for a configured second factor. Choose
an authenticator-app code, passkey, or recovery code from the available
methods.

The login page also supports passwordless passkey sign-in through the dedicated
WebAuthn login flow. The separate WebAuthn authentication flow confirms a
signed-in user's identity before a protected action.

## Manage existing methods

The **Two-Factor Authentication** card on **Profile** shows the current methods.
From there you can:

- disable the authenticator app;
- add or remove passkeys;
- view the remaining recovery-code count; and
- replace the recovery-code set.

MedTracker asks for a password or fresh second-factor check before sensitive
changes. Removing one method does not remove the others.

## Recover access

If one method is unavailable, use another configured method. After signing in,
remove the lost credential and add its replacement.

If every method is unavailable, use account recovery or contact the deployment
administrator. An administrator should verify the account holder before
changing access.

## Deployment notes

Passkeys use `APP_URL` as their origin and relying-party source. Production
requires an HTTPS URL with the public host. See the [passkey
guide](passkey-setup.md) before changing a live hostname.

MedTracker requires passkey user verification and discoverable credentials. It
does not request direct authenticator attestation.

Authentication setup, successful checks, failures, and credential removal are
written to the audit trail. Secret values must not be included in application
logs.
