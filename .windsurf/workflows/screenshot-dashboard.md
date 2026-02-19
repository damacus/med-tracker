---
description: Take screenshots of pages changed in the current pull request
---

# Screenshot Changed Pages

Take before/after screenshots of any pages affected by the current PR, at desktop and mobile viewports.

## Prerequisites

The dev environment must be running. If not, start it first:

```bash
task dev:up
```

## Steps

1. Identify which pages were changed in this PR:

   ```bash
   git diff origin/main...HEAD --name-only
   ```

   Map changed files to their URLs (e.g. `app/components/dashboard/` → `/`, `app/components/people/` → `/people`, etc.)

1. Open the browser and log in:

   ```bash
   playwright-cli open http://localhost:$(task dev:port)/login
   playwright-cli snapshot
   ```

   Fill credentials using refs from the snapshot:

   ```bash
   playwright-cli fill <email-ref> "nurse.smith@example.com"
   playwright-cli fill <password-ref> "password"
   playwright-cli click <sign-in-button-ref>
   ```

   If redirected to `/otp-auth`, sign out and use a different fixture user (see Notes).

1. For each changed page, take desktop and mobile screenshots. Replace `<page>` with a short slug (e.g. `dashboard`, `people`, `person-show`):

   **Desktop (1440×900):**

   ```bash
   playwright-cli resize 1440 900
   playwright-cli goto http://localhost:$(task dev:port)/<path>
   playwright-cli screenshot --filename=docs/screenshots/<page>-desktop.png
   ```

   **Full page:**

   ```bash
   playwright-cli run-code "async page => { await page.screenshot({ path: 'docs/screenshots/<page>-desktop-full.png', fullPage: true, scale: 'css', type: 'png' }); }"
   ```

   **Mobile (390×844 — iPhone 14 Pro):**

   ```bash
   playwright-cli resize 390 844
   playwright-cli screenshot --filename=docs/screenshots/<page>-mobile.png
   ```

1. Close the browser:

   ```bash
   playwright-cli close
   ```

## Output

Screenshots are saved to `docs/screenshots/` using the naming convention:

- `<page>-desktop.png` — 1440×900 viewport
- `<page>-desktop-full.png` — full page scroll
- `<page>-mobile.png` — 390×844

## Notes

- All fixture users have password `password`
- `nurse.smith@example.com` (nurse) — no MFA, sees all patients
- `bob.smith@example.com` (carer) — no MFA, sees assigned patients
- `damacus@example.com` (admin) — has MFA enabled, requires TOTP
- Screenshots directory is gitignored; commit selectively if needed
