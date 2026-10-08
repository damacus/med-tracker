# Profile page parity subplan

Recorded 7 October 2026. Parent: [browser delivery](06-browser.md).

## Outcome and audit limits

Port the entire Rails profile layout and its reachable items to Loco, preserving
tabs, sheets, dialogs, expandable groups, ordering and behaviour in the current
themes. The owner explicitly rejected the simplified draft as the target.
Appearance belongs within Profile settings, not only in the global header.
Do not count a link, placeholder or saved preference with no effect as parity.

This is a quick source and existing-test audit, not a rendered visual comparison.
Reference: `rails/app/views/profiles/` in the retained Rails application.
Candidate: `assets/views/profile/show.html` and `src/controllers/profile.rs` in
the local `loco-profile` worktree. The candidate remains unpublished and incomplete.
Security services may exist elsewhere; absence below means absent from this
profile composition, not necessarily absent from the whole application.

## Item-by-item inventory

| ID | Rails item and interaction | Draft comparison and required work |
| --- | --- | --- |
| P01 | Hero: avatar, profile title, email and description | Plain heading/avatar upload currently; restore composed identity header |
| P02 | Four tabs in order: Profile, Security, Notifications, Advanced; active section and associated panels | Three links, two anchors and external Security destination; implement actual four-tab navigation |
| P03 | Section titles and current-state summaries | Restore summaries in each panel |
| P04 | Personal information card: name, email, time zone, date of birth, conditional age, person type, capacity and unset values | Draft exposes date-of-birth/time-zone inputs; restore all display rows without broadening edit permissions |
| P05 | Desktop two-column Profile layout, stacked responsive layout | Restore personal information/settings relationship |
| P06 | Avatar sheet: current identity, supported formats, upload, conditional removal, save/errors and close | Operations partly implemented inline; port sheet and preserve storage/cleanup verification gates |
| P07 | Gravatar preference and explanation within avatar sheet | Preference exists; rendering remains subject to the existing external-hash permission block |
| P08 | Time-zone dialog with selection, save, cancel/close and error feedback | Inline selector exists; port interaction |
| P09 | Mobile navigation shortcuts: three ordered slots, None choice and permitted destinations | Draft uses unordered checkboxes; restore ordered-slot behaviour. These are not keyboard shortcuts |
| P10 | Appearance sheet: Light, Dark, System; selected states and immediate application | Existing shared Appearance control; integrate in Profile and reconcile header entry point |
| P11 | Ten palettes: Command Centre, Serene Sage, Modern Clinical, Warm Earth, Deep Lavender, Forest Care, Sunset Support, Tech Indigo, Soft Rose, Minty Fresh | Reuse current theme implementation; verify every palette, mode, persistence and first paint |
| P12 | Security account card: change email and change password with modal navigation | Missing composition; consume auth-owner routes and outcomes |
| P13 | TOTP status and enable/disable actions | Missing composition; auth owner retains protocol and sensitive operations |
| P14 | Recovery-code status/count and available management action | Missing composition; integrate verified auth-owned flow |
| P15 | Passkey list, empty state, added date, add and remove actions | Missing composition; integrate verified auth-owned flow |
| P16 | Browser push status, enable/disable and send-test action | Absent from draft; implement working subscription/status/test journey with existing worker contracts |
| P17 | Reminder master switch and dose-due, missed-dose, low-stock and private-text preferences, descriptions and save | Five fields exist; restore grouping, controls and save/error behaviour |
| P18 | Expandable delivery-time section: morning, afternoon, evening, night | Inputs exist inline; restore disclosure and persisted values |
| P19 | Expandable managed-people section: adult opt-in controls and automatic dependent coverage indicator | Draft lists adult checkboxes; restore conditional people presentation and permission rules |
| P20 | Advanced accordion: API tokens, data backup, experiments, system information | Entire panel absent |
| P21 | API token creation/name, one-time secret, empty/list states, last-used and expiry information, revoke | Integrate auth-owned token service; preserve membership scope and secret handling |
| P22 | Data backup: Health data JSON and unencrypted ZIP downloads, privacy warning | Missing; browser/domain delivery owns exports after auth prerequisite. Old byte-for-byte export compatibility remains excluded |
| P23 | Experiments: Add Medication wizard Full page/Modal/Slide-over | Missing; inventory and implement actual affected behaviour, not inert choices |
| P24 | Experiments: dashboard Current/Time first/Family lanes/Calm focus | Missing; retain as explicit parity work and verify destination behaviour |
| P25 | Experiments: medication launcher Current/Context-aware | Missing; selected-person context and global entry behaviour must match |
| P26 | System information: development worktree/commit, otherwise version/release notes; documentation link | Missing; show honest Loco build metadata with same conditional presentation |
| P27 | Separate Danger Zone: close-account confirmation, cancel, password and submit | Missing; integrate auth/lifecycle owner service after prerequisite, no live account mutation |
| P28 | Localized copy and accessible names, roles, states, panel relationships, errors and announcements | Draft contains hardcoded English; repair together with visible translation gaps |

References: `show.rb`, `profile_settings.rb`, `personal_info_content.rb`,
`security_section_content.rb`, `account_security_card.rb`, `two_factor_card.rb`,
`notifications_card.rb`, `notification_delivery_settings.rb`,
`managed_notification_people.rb`, `advanced_section_content.rb`,
`api_tokens_card.rb`, `data_exports_card.rb`, `experiments_card.rb`,
`version_info.rb`, `danger_zone_card.rb` and `close_account_dialog.rb`.
Follow their rendered descendants rather than treating unused files as UI items.

## Implementation sequence and ownership

1. Capture rendered Rails reference states for all four tabs and every sheet,
   dialog and accordion above, desktop and mobile. Reconcile conditional states
   against source and tests; expand this inventory if rendering exposes omissions.
2. Write genuine failing browser parity tests for P01–P11, then port shell and
   Profile settings in the existing profile worktree. Preserve existing changes;
   one implementation writer owns profile templates, controller and focused tests.
3. Complete notification presentation and missing push operations, P16–P19.
   Preserve consent, household/person authorization and delivery failure feedback.
4. Integrate Security and API-token presentation against accepted auth interfaces.
   Do not reimplement authentication or edit the independent auth worktree.
5. Complete Advanced, exports and close-account integration in the established
   auth-first lifecycle sequence. Track experiments' real destination behaviour
   explicitly; none of P23–P25 may disappear silently from the parity checklist.
6. Find and fix keyboard, semantic, localization and responsive defects across all
   states, then independently review the coherent candidate. Run required checks,
   measured full CI and publish reviewed PRs. No merge or deployment.

## Acceptance checks

- [ ] Every P01–P28 item has a working implementation and evidence, or an explicit
  unresolved dependency. Dependencies are not passing parity results.
- [ ] Actual tabs expose name/role/selection and panel relationships, support
  keyboard navigation and survive save/validation redirects in the correct panel.
  Port the Rails 320px/enlarged-text regression from
  `rails/spec/system/profile_tabs_spec.rb`; inspect current tab behaviour before
  choosing maintained Loco-compatible components.
- [ ] Sheets/dialogs/disclosures work with keyboard alone; opening/closing,
  Escape where applicable, focus containment/return, expanded state, error focus,
  hidden-panel focus exclusion and unsaved-input preservation are verified.
- [ ] Keyboard shortcuts are inventoried separately from mobile shortcuts; verify
  discoverability, conflicts and editable-field handling for supported shortcuts.
- [ ] Save/reload, invalid input, denied role, revoked access, empty and populated
  states are exercised with synthetic records. Test real operation effects.
- [ ] Theme selection works from Profile across all ten palettes and three modes;
  desktop/mobile screenshots show each relevant layout state and representative
  light/dark themes. Check contrast, zoom/reflow and target sizes.
- [ ] Selected language reaches visible copy, accessible names/hints/errors/live
  announcements and the document language. Preserve native semantics; avoid
  unnecessary ARIA or accessible names inconsistent with visible labels.
- [ ] Use `frontend-design`, `ui-professional`, `translate`, `accessibility` and
  `web-quality-audit` for implementation and evidence-led correction. Per the
  owner, a dedicated screen-reader session is not required. Inspect the rendered
  accessibility tree and automate interaction checks; do not claim tested
  screen-reader compatibility or whole-app conformance from Lighthouse scores.

## Current gaps and next action

The quick audit establishes substantial layout and feature gaps; it does not
invalidate working profile model/API tests. Reuse those operations and the
existing theme engine. Finish avatar active-key cleanup/retry verification and
other preserved profile prerequisites alongside their related implementation.
The next step is the rendered reference capture and the first failing four-tab
parity test, not another simplified profile design.
