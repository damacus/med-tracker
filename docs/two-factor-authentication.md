# Authentication and account recovery

Loco uses Better Auth's native authentication store and plugins. Open **Account
security** to manage passwords, passkeys, authenticator enrolment and sessions.
Authentication starts fresh; there is no migration of Rails passwords, tokens,
passkeys or authenticator enrolments. The Rails application under `rails/` is a
separate reference and does not share Loco sessions.

## Choose your methods

- A **password** authenticates the first sign-in step.
- A **passkey** uses a device, password manager or security key and supports
  passwordless sign-in.
- An **authenticator app** supplies the second-factor code when MFA is enabled.
- **MFA backup codes** replace an unavailable authenticator code after the first
  sign-in step. A backup code alone cannot sign in.

Authenticator enrolment is optional, including for household owners and
administrators. Keep an independent way to regain access before removing a
credential.

## Set up an authenticator app

1. Sign in and open **Account security**.
2. Start authenticator setup and confirm your current password. Add a local
   password first if the account does not have one.
3. Add the displayed setup key to a compatible authenticator app.
4. Save the displayed native MFA backup codes outside MedTracker and acknowledge
   that they are saved.
5. Enter the current code to confirm setup.

The code changes every 30 seconds. Name the authenticator entry **MedTracker**
when entering the setup key manually. Use the native confirmation flow to complete enrolment;
starting setup alone does not enable MFA.

## Add a passkey

Open **Account security** and follow the passkey setup prompt, then the browser
or password-manager prompt. Give the passkey a name that identifies where it is
stored.

Passkeys use the configured public application origin and require HTTPS outside
local development. See the [passkey guide](passkey-setup.md) for hostname and
device requirements.

## Sign in and recover access

After password sign-in, complete the native MFA challenge when the account has
MFA enabled. Use an authenticator code or one of the account's native backup
codes. Each backup code works once; generating a replacement set invalidates the
old set. Keep these codes outside the device used for the authenticator.

Passwordless passkey sign-in remains available. If a password is unavailable,
use the email password-recovery flow and follow the link sent to the account's
registered address. Password recovery does not remove configured MFA.

Standalone recovery-code sign-in is deliberately unsupported. If no configured
method or backup code remains usable, contact the deployment administrator;
there is no application recovery-code bypass of the native authentication flow.

## Manage existing methods

Use **Account security** to manage credentials and revoke sessions. Disabling
MFA requires your current password through Better Auth’s native confirmation
flow. Complete the requested fresh authentication before other sensitive changes. Removing a credential
does not grant or remove household permissions. Locked or closed accounts and
incomplete enrolment sessions cannot use credential-management routes to regain
clinical access.

Authentication events record the affected account or session. Trusted HTTP actor
and request events can be recorded separately. Clinical and household audit
requirements remain unchanged. Secret values must not appear in application
logs, error responses or account-security inventories.

## Session lifetime

Loco web and PWA sessions expire after seven days of inactivity or 30 days from
sign-in, whichever comes first. Normal authenticated use renews the idle window
without extending the maximum age. Mobile OAuth defaults to the same limits and
permits shorter settings:

| Variable | Default | Meaning |
| --- | --- | --- |
| `SESSION_INACTIVITY_TIMEOUT_DAYS` | `7` | Mobile inactivity limit, from 1 to 7 whole days. |
| `SESSION_MAX_AGE_DAYS` | `30` | Mobile absolute login limit, from 1 to 30 whole days; it cannot be disabled. |

Refreshing a mobile access token preserves the original login time and does not
count as user activity. Expired or revoked logins must authenticate again.
Create personal API keys from Account security while signed in. Choose their
households, permissions and an expiry from 1 to 365 days. Creation uses the native
authenticated session without an additional password or passkey challenge. Using a
key does not extend its expiry. Keys are separate from MFA backup codes and
account recovery.
