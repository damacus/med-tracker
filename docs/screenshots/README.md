# Screenshots for 2FA Management PR

## Required Screenshots

To complete the GitHub PR, please take the following screenshots manually:

### 1. Profile Page - Full 2FA Management Card

**URL:** `http://localhost:3000/profile` (after logging in as damacus@example.com)

**What to capture:**
- The entire "Two-Factor Authentication" card showing all three sections:
  - Authenticator App (TOTP) - showing "Not configured" state
  - Recovery Codes - showing "Not generated" state  
  - Passkeys - showing any registered passkeys

**Filename:** `profile-2fa-overview.png`

### 2. TOTP Setup Page

**URL:** `http://localhost:3000/otp-setup`

**What to capture:**
- QR code for authenticator app setup
- Manual entry code option
- Setup form

**Filename:** `totp-setup.png`

### 3. Recovery Codes Page

**URL:** `http://localhost:3000/recovery-codes`

**What to capture:**
- List of generated recovery codes
- Warning messages about saving codes securely

**Filename:** `recovery-codes.png`

### 4. Passkey Setup Flow

**URL:** `http://localhost:3000/webauthn-setup`

**What to capture:**
- Browser's passkey creation dialog (if possible)
- Or the setup page before the browser dialog appears

**Filename:** `passkey-setup.png`

### 5. Profile Page - With 2FA Enabled

**What to capture:**
- Same view as #1 but with:
  - TOTP showing "Enabled" status
  - Recovery codes showing count
  - Multiple passkeys listed

**Filename:** `profile-2fa-enabled.png`

## How to Take Screenshots

1. Start the development server:
   ```bash
   docker compose -f docker-compose.dev.yml up -d
   ```

2. Log in with test credentials:
   - Email: `damacus@example.com`
   - Password: `password`

3. Navigate to each URL and capture screenshots

4. Save screenshots to this directory (`docs/screenshots/`)

## Screenshot Guidelines

- Use a viewport size of approximately 1280x1024
- Capture full page or relevant card sections
- Ensure text is readable
- Include browser chrome if showing native dialogs (like passkey setup)
- Use PNG format for clarity
