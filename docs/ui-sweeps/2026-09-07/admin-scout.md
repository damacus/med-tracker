# Admin, signed-out, and global navigation scout

Baseline: `45db2112` on `codex/fix-web-ui-sweep`.

This is a read-only source sweep. No tests, browser, Canary, or local server were run. Findings marked as source facts are confirmed from the current checkout; visual outcomes remain reproduction candidates until the coordinator runs the local browser matrix.

## Findings

### 1. Admin users index has hard-coded English heading and action label

- Severity: P2, wording/localisation
- Route/role/state: `/admin/users`, authenticated administrator, any non-English locale; empty or populated list does not affect the issue.
- Source fact: `app/components/admin/users/index_view.rb:51-64` emits `User Management` and `New User` as literals. The same file uses `admin.users.index.helper` for the helper text, and `config/locales/en.yml:226-246` has the surrounding users/form translation tree. Portuguese and Welsh have corresponding `admin.users` trees, so these two labels bypass the locale contract.
- Reproduction candidate: sign in as an administrator, set `I18n.locale` to `:pt` or `:cy`, visit `admin_users_path`, and compare the page heading/action with the translated surrounding copy. Expected: translated heading and action; source predicts the literal English strings.
- Likely impact: mixed-language page and inconsistent terminology in the primary admin user-management workflow. This is a source-confirmed wording defect, not a confirmed screenshot result.

### 2. Invitation form and recent rows fall back to English/titleized enum names

- Severity: P2, wording/localisation
- Route/role/state: `/admin/invitations`, authenticated administrator, non-English locale, with the form visible and at least one existing invitation.
- Source fact: `app/components/admin/invitations/index_view.rb:103-145` renders membership roles, relationship types, and access levels with `role.titleize`, `relationship_type.titleize`, and `access_level.titleize`; labels for relationship/access also use English defaults at lines 120-140. The recent invitation row at lines 209-215 titleizes the membership role. The locale trees currently provide translated invitation page labels/statuses (`config/locales/en.yml:121-140`, `config/locales/pt.yml:384-404`, `config/locales/cy.yml:377-397`) but no matching keys for these option values/default labels.
- Reproduction candidate: sign in as an administrator, set `I18n.locale` to `:pt` or `:cy`, visit `admin_invitations_path` with a pending invitation and open each select. Expected: translated field labels and option values; source predicts English labels such as `Dependent relationship`, `Select relationship`, `Dependent access`, and titleized English enum values.
- Likely impact: administrators may see a partially untranslated form and less clear relationship/access terminology. This is a source-confirmed wording risk; no browser capture was taken.

### 3. Mobile rail gives every label a two-line box without a narrow-width overflow guard

- Severity: P2, narrow viewport/readability candidate
- Route/role/state: any authenticated household route that renders the mobile rail; 390px or 320px viewport; account configured with a longer shortcut such as finder, medicine reviews, or administration; translated locale is a useful stress state.
- Source fact: `app/components/layouts/mobile_rail.rb:42-62` gives each item `flex-1`, a fixed `h-7 w-16` icon container, and a label span with `h-[2.5em] text-xs leading-tight`, but no `min-w-0`, word breaking, line clamp, or overflow handling. `app/models/account.rb:11-14` permits up to three shortcuts including `medicine_reviews` and `administration`, while `app/components/layouts/navigation_items.rb:31-45` supplies their full labels.
- Reproduction candidate: at 320px and 390px, authenticate, save three long shortcuts (for example `medicine_reviews`, `administration`, and `finder`), switch between English, Portuguese, and Welsh, and inspect each label against the 80px rail bounds. Expected: labels remain readable and contained; source predicts wrapping/overlap or clipping pressure for long translations. This requires browser geometry evidence before acceptance.

### 4. Admin invitation rows have no explicit long-email break strategy on the narrow card layout

- Severity: P2, narrow viewport/readability candidate
- Route/role/state: `/admin/invitations`, authenticated administrator, 320px/390px viewport, existing invitation with a long email and action buttons.
- Source fact: `app/components/admin/invitations/index_view.rb:209-240` puts the email in a plain `p` and the row in `flex flex-col md:flex-row`; the email block has no `min-w-0`, `break-words`, or `break-all`, while action controls are `shrink-0` at lines 226-240. The current mobile audit route list includes `admin_invitations_path` (`spec/system/mobile_ui_audit_spec.rb:212-227`) but does not inject a long-email state.
- Reproduction candidate: create an invitation whose address is long enough to exceed the card width, visit the route at 320px and 390px in both themes, and measure page/card overflow plus action containment. Expected: the email wraps inside the card and actions remain usable; source leaves the outcome unverified.

## Route matrix gaps

Existing `spec/system/mobile_ui_audit_spec.rb:49-79` covers signed-out `login_path`, `create_account_path`, and `accept_invitation_path` at 390px, and `:67-79` covers login/create-account at 1280px and 390px. It does not cover reset-password request/submit, verification resend/verify, unlock-account, passkey/WebAuthn, OTP/MFA, recovery-code, or change-login/password surfaces.

The admin route list at `spec/system/mobile_ui_audit_spec.rb:212-227` covers dashboard, NHS import new, users index/new, invitations index, carer relationships index/new, people index, audit index/show, and settings. It omits household edit, user edit and member actions (activate, verify, destroy, membership-role update), invitation resend/cancel states, carer activate/destroy states, and the ambiguous-person-access-grants index. Those are route/state gaps, not claims that the omitted screens are defective.

Global search has request coverage and desktop-oriented interaction/geometry coverage in `spec/system/global_search_spec.rb`, but the current route audit does not exercise the palette at 320px/390px in light and dark modes with long result titles, no results, and keyboard selection. The signed-out audit correctly excludes it because the application shell only renders it for authenticated users.

## Unavailable / not run

- Local browser/server and screenshots: unavailable by scout contract; coordinator owns reproduction.
- Canary: excluded by `docs/ui-sweeps/2026-09-07/plan.md`.
- Tests and lint: intentionally not run.
