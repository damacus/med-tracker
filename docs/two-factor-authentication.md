# Two-Factor Authentication

The enrolment instructions below describe the independently retained Rails
rollback application. Loco uses Better Auth through **Account security** and
requires replacement of historical passkeys at cutover. Supported retained
passwords and authenticator enrolments remain available.
Its session policy is documented in the session-lifetime section below.

MedTracker supports authenticator-app codes, passkeys, and recovery codes.
Enrolment is optional, including for household owners and administrators.
Once configured, Rodauth applies the account's sign-in requirements.

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
credential changes. Removing one method does not remove the others.

Household and platform administration use the current login and permissions.
They do not impose an extra 15-minute MFA deadline.

## Session lifetime

Loco web and PWA sessions expire after seven days of inactivity or 30 days
from sign-in, whichever comes first. Normal authenticated use renews the idle
window without extending the maximum age. Better Auth uses these fixed browser
limits; mobile OAuth defaults to the same limits and permits shorter settings:

| Variable | Default | Meaning |
| --- | --- | --- |
| `SESSION_INACTIVITY_TIMEOUT_DAYS` | `7` | Mobile inactivity limit, from 1 to 7 whole days. |
| `SESSION_MAX_AGE_DAYS` | `30` | Mobile absolute login limit, from 1 to 30 whole days; it cannot be disabled. |

Refreshing a mobile access token preserves the original login time and does not
count as user activity. Expired or revoked Loco logins must sign in through
Better Auth again. Personal API keys have an explicit expiry from 1 to 365 days;
using a key does not extend it.

The independently preserved Rails rollback application retains its own session
and legacy API-token policies. It does not provide session continuity with Loco.

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
