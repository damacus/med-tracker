# Two-Factor Authentication (2FA)

MedTracker supports multiple two-factor authentication methods to secure user accounts. This document describes the available 2FA options and how to manage them.

## Available 2FA Methods

### 1. Authenticator App (TOTP)

Time-based One-Time Password (TOTP) authentication using apps like:

- Google Authenticator
- Microsoft Authenticator
- 1Password
- Authy
- Any TOTP-compatible authenticator app

**Setup:**

1. Navigate to your profile page
2. In the "Two-Factor Authentication" section, click "Set up authenticator app"
3. Scan the QR code with your authenticator app
4. Enter the 6-digit code from your app to confirm setup

**Usage:**

- After entering your password during login, you'll be prompted for a 6-digit code
- Open your authenticator app and enter the current code
- Codes refresh every 30 seconds

**Management:**

- To disable TOTP, visit your profile and click "Disable" in the Authenticator App section
- You can only have one TOTP configuration per account

### 2. Recovery Codes

Recovery codes are one-time use backup codes that allow you to access your account if you lose access to your primary 2FA method.

**Setup:**

1. Navigate to your profile page
2. In the "Two-Factor Authentication" section, click "Generate recovery codes"
3. Save the codes in a secure location (password manager, encrypted file, etc.)
4. Each code can only be used once

**Important:**

- Store recovery codes securely - treat them like passwords
- Each code can only be used once
- Generate new codes if you've used several or suspect they've been compromised
- Regenerating codes invalidates all previous codes

**Usage:**

- During login, if you can't access your authenticator app or passkey, click "Use recovery code"
- Enter one of your recovery codes
- The code will be marked as used and cannot be reused

### 3. Passkeys (WebAuthn)

Passkeys provide passwordless authentication using biometrics or security keys.

**Supported Methods:**

- Touch ID / Face ID (macOS, iOS)
- Windows Hello (Windows)
- Hardware security keys (YubiKey, etc.)
- Android biometrics

**Setup:**

1. Navigate to your profile page
2. In the "Two-Factor Authentication" section, click "Add a passkey"
3. Follow your browser's prompts to create a passkey
4. Give your passkey a memorable name (e.g., "MacBook Pro Touch ID", "YubiKey")

**Usage:**

- During login, you can authenticate using your passkey instead of a password
- Simply click "Sign in with passkey" and follow your device's prompts

**Management:**

- You can register multiple passkeys (e.g., one for each device)
- Remove passkeys you no longer use from your profile page
- Each passkey is tied to a specific device or security key

## Managing 2FA on Your Profile

All 2FA methods can be managed from your profile page (`/profile`):

1. **View Status:** See which 2FA methods are enabled
2. **Add Methods:** Set up new 2FA methods
3. **Remove Methods:** Disable or remove existing 2FA methods
4. **View Recovery Codes:** Access your recovery codes (if generated)
5. **Regenerate Recovery Codes:** Create new recovery codes (invalidates old ones)

## Security Best Practices

1. **Enable Multiple Methods:** Use both TOTP and passkeys for redundancy
2. **Generate Recovery Codes:** Always have recovery codes as a backup
3. **Store Codes Securely:** Keep recovery codes in a password manager or encrypted storage
4. **Register Multiple Passkeys:** Add passkeys for multiple devices
5. **Review Regularly:** Periodically review and remove unused passkeys
6. **Update After Device Changes:** Remove passkeys for devices you no longer own

## Troubleshooting

### Lost Access to Authenticator App

1. Use a recovery code to log in
2. Disable TOTP from your profile
3. Set up TOTP again with a new device

### Lost Passkey Device

1. Log in using password + TOTP or recovery code
2. Remove the lost passkey from your profile
3. Add a new passkey for your current device

### Used All Recovery Codes

1. Log in using password + TOTP or passkey
2. Generate new recovery codes from your profile
3. Save the new codes securely

### Can't Access Any 2FA Method

Contact your system administrator for account recovery assistance.

## Technical Details

### TOTP Configuration

- **Algorithm:** SHA-1
- **Digits:** 6
- **Period:** 30 seconds
- **Issuer:** MedTracker

### WebAuthn Configuration

- **RP Name:** MedTracker
- **RP ID:** localhost (development), medtracker.com (production)
- **Attestation:** Direct
- **User Verification:** Preferred
- **Authenticator Attachment:** Platform or cross-platform

### Recovery Codes

- **Format:** 16-character alphanumeric codes
- **Quantity:** 10 codes generated per set
- **Usage:** Single-use only
- **Storage:** Encrypted in database

## API Routes

The following Rodauth routes are available for 2FA management:

- `/otp-setup` - Set up TOTP authentication
- `/otp-auth` - Authenticate with TOTP code
- `/otp-disable` - Disable TOTP authentication
- `/recovery-codes` - View/generate recovery codes
- `/webauthn-setup` - Set up a new passkey
- `/webauthn-auth` - Authenticate with passkey
- `/webauthn-remove` - Remove a passkey
- `/multifactor-manage` - Manage all 2FA methods
- `/multifactor-auth` - Choose 2FA method during login

## Database Schema

### account_otp_keys

Stores TOTP secrets for each account.

### account_recovery_codes

Stores recovery codes (hashed) for each account.

### account_webauthn_keys

Stores WebAuthn credentials (passkeys) for each account.

### account_webauthn_user_ids

Maps WebAuthn user IDs to accounts.
