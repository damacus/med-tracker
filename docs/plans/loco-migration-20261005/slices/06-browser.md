# Complete browser workflows and themes Implementation Plan

> For agentic workers: retain the persistent team and Devin review from `../team-charter.md`.

**Goal:** Replace the legacy browser application with complete, accessible Tera/daisyUI journeys.
**Architecture:** Browser controllers call accepted shared operations and render Tera projections.
Small application scripts own interaction and offline outbox behaviour; CSS owns appearance.
**Tech Stack:** Loco/Tera, daisyUI theme exports, maintained browser tooling and service-worker APIs.
**Spec:** [implementation-spec.md](../implementation-spec.md), captured profile/notification/theme/font PR inputs.

## Global constraints

Implement each journey against its locally verified operation interface. Complete
care/identity and relevant API/workers remain acceptance requirements, rather than
blocking independent shell/theme or journey implementation. The user's Tera/daisyUI migration
instruction supersedes Rails-only Phlex rules for the Loco UI; retain Phlex within Rails rollback.
Theme exports preserve values/names, licensed fonts and both storage keys. No read-only journey substitute.

## Review focus

Initial paint matches saved/system appearance (B1). Keyboard focus returns after dialogs/tabs (B1).
Failed mutations preserve input and show associated errors (B2). All notification switches persist (B2).
Offline replay after permission revocation cannot write, and reconnect retries cannot double-dose (B3).

### B1: Establish the accessible shell and existing themes

**Files:** Create `assets/views/layouts/base.html`, `assets/views/components/{tabs,dialog,form_errors}.html`,
`assets/static/css/{app,themes}.css`, `assets/static/js/{appearance,interaction}.js`,
`tests/browser/shell.spec.mjs`, `scripts/migration/loco-browser.mjs`; modify `Taskfile.yml` and view initializer.
**Interfaces:** Produce `slice:browser GROUP=<group>` with actual owned Loco, desktop/mobile viewports
and screenshots. `appearance.js` reads/writes `med-tracker-theme` and `med-tracker-appearance`;
rendered pages provide associated labels/errors and semantic controls.

- [ ] Test every existing palette and Light/Dark/System, including before first paint and live OS change;
  test keyboard tabs/dialog focus. Assert `saved_palette_restored == true`,
  `system_change_applied == true`, `focus_after_close == triggering_control`.
- [ ] Register the browser Task with its test and run `rtk task slice:browser GROUP=shell`; record RED.
- [ ] Export existing themes with the official daisyUI theme creator; commit CSS directly.
  Preserve fonts/licences from captured inputs; implement accessible interaction with maintained patterns.
- [ ] Run shell checks at desktop/mobile and capture `docs/screenshots/loco-shell-{desktop,mobile}.png`;
  verify contrast, no horizontal overflow, focus and initial appearance in the actual UI.
- [ ] Review and commit `feat(ui): preserve accessible shell and existing themes`.

### B2: Port complete browser operations by journey family

**Files:** Create `src/controllers/browser.rs`, `src/controllers/browser/{care,profile,admin,data}.rs`,
`assets/views/{care,profile,admin,data}/` templates and browser tests listed below; modify route registration.
**Interfaces:** Each controller exposes `routes() -> Routes`; mutations consume the same care/integration
`Command` types and validated actor/scope. Views receive preloaded projections, never query the database.

Each row is an independently reviewed task using the five checkbox steps below. Complete all rows;
a navigation link or list page is not a successful write journey.

| Task | Templates/test file | Complete operation and acceptance assertions |
| --- | --- | --- |
| B2a Care | `care/`; `tests/browser/care.spec.mjs` | Create/invite household, accept invitation, create/grant person, medicine/dosage/stock order/receipt, schedule/assignment/pause/resume, take/correct dose; exact stock/audit once; role/capacity denial; supported person-scoped schedule edit |
| B2b Profile | `profile/`; `tests/browser/profile.spec.mjs` | All four profile tabs; edit details/avatar, password/MFA/passkeys, API tokens/sessions/devices, all five notification preferences; validation retains input; revocation takes effect immediately |
| B2c Admin/support | `admin/`; `tests/browser/admin.spec.mjs` | Create/update/retire managed records, duplicate/validation handling, immediately usable records, timed support access, owner promotion and household retirement with denied-role cases |
| B2d Data | `data/`; `tests/browser/data.spec.mjs` | Lookup/reviews, import status/retry, authorised export/history/PDF download and signed attachment flows; external failure/expiry shown accessibly; report timezone boundaries |

- [ ] Write each row's successful write, invalid input and denied-role browser cases; inspect actual
  persisted state. Assertions include `mutation_visible_after_reload == true`,
  `duplicate_side_effects == 0`, `denied_mutation_committed == false`.
- [ ] Run `rtk task slice:browser GROUP=<care|profile|admin|data>` for the row and record the missing journey.
- [ ] Implement that row's standard routes/controllers/templates via accepted shared operations;
  preserve existing labels, contract routing and error feedback. Never bypass permissions for rendering.
- [ ] Run the row's desktop/mobile browser checks, relevant domain tests and root CI; capture actual
  `docs/screenshots/loco-<group>-{desktop,mobile}.png` and verify keyboard operation.
- [ ] Review and commit each row independently; update its evidence in the HTML report when accepted.

### B3: Preserve offline dose capture, replay and live updates

**Files:** Create `assets/static/{service-worker.js,manifest.webmanifest}`, `assets/static/js/offline.js`,
`tests/browser/offline.spec.mjs`; modify care templates for pending/conflict feedback.
Read existing Rails PWA/offline JavaScript and sync/replay contracts.
**Interfaces:** Offline outbox stores the existing client UUID/payload identity and sends through C3's
accepted replay API. Personal caches/outbox are isolated by account and cleared according to existing logout policy.

- [ ] Test airplane-mode capture, browser restart, reconnect twice, permission withdrawal before upload,
  account switch and partial conflict. Assert `dose_count_after_two_replays == 1`,
  `revoked_pending_write_committed == false`, `other_account_sees_cached_health_data == false`.
- [ ] Run `rtk task slice:browser GROUP=offline`; record missing/offline safety failures.
- [ ] Implement service-worker cache/outbox/replay using proven browser capabilities and C3 contracts;
  retain pending/conflict states and existing realtime behaviour from source inventory.
- [ ] Run restart/offline tests, API replay tests and full browser suite; verify real UI feedback/screenshots.
- [ ] Obtain privacy/replay review, integrate and publish; reconcile every browser capability before completion.

**Done:** All journey families, appearance, accessibility, security settings, notifications and offline
replay pass actual desktop/mobile UI checks through the Loco server.
