---
description: Take screenshots of the dashboard at desktop and mobile viewports
---

# Screenshot Dashboard

## Prerequisites

The dev environment must be running. If not, start it first:

```bash
task dev:up
```

Find the dev server port:

```bash
task dev:ps
```

Note the host port mapped to container port 3000 (e.g. `0.0.0.0:51125->3000/tcp`).

## Steps

1. Open the browser and navigate to the login page (replace PORT with the actual port):

   ```bash
   playwright-cli open http://localhost:PORT/login
   ```

1. Take a snapshot to get the current element refs:

   ```bash
   playwright-cli snapshot
   ```

1. Fill in credentials using the refs from the snapshot (email ref and password ref):

   ```bash
   playwright-cli fill <email-ref> "nurse.smith@example.com"
   playwright-cli fill <password-ref> "password"
   playwright-cli click <sign-in-button-ref>
   ```

1. Verify you landed on `/dashboard` (check the Page URL in the output). If redirected to `/otp-auth`, sign out and use a different fixture user without MFA.

1. Set desktop viewport and take screenshot:

   ```bash
   playwright-cli resize 1440 900
   playwright-cli screenshot --filename=docs/screenshots/dashboard-desktop.png
   ```

1. Take a full-page screenshot:

   ```bash
   playwright-cli run-code "async page => { await page.screenshot({ path: 'docs/screenshots/dashboard-desktop-full.png', fullPage: true, scale: 'css', type: 'png' }); }"
   ```

1. Set mobile viewport and take screenshot:

   ```bash
   playwright-cli resize 390 844
   playwright-cli screenshot --filename=docs/screenshots/dashboard-mobile.png
   ```

1. Close the browser:

   ```bash
   playwright-cli close
   ```

## Output

Screenshots are saved to `docs/screenshots/`:

- `dashboard-desktop.png` — 1440×900 viewport
- `dashboard-desktop-full.png` — full page scroll
- `dashboard-mobile.png` — 390×844 (iPhone 14 Pro)

## Notes

- All fixture users have password `password`
- `nurse.smith@example.com` (nurse) — no MFA, sees all patients
- `bob.smith@example.com` (carer) — no MFA, sees assigned patients
- `damacus@example.com` (admin) — has MFA enabled, requires TOTP
- Screenshots directory is gitignored; commit selectively if needed
