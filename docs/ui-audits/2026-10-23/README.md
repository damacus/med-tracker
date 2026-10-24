# Browser UI parity and usability audit

Audit completed, 23–24 October 2026. The main care journeys work in the
synthetic browser, but sign-in hierarchy, the chooser security link, schedule
feedback and dashboard keyboard cancellation need bounded corrections.
This report changes no application behaviour and grants no merge/deployment approval.

223 accepted screenshots and 8 rejected captures are indexed.
All 26 page families and 159 route declarations are accounted for. This is a
completed audit with explicit limits, not verification of every role, populated
state, provider, device or download. See the [route ledger](route-coverage.csv)
and [ordered screenshots](captures.md). Valid OAuth consent rendering, PDFs,
three saved dashboard variants and several fixture-dependent states remain
unverified, as detailed below.

## Scope and evidence

Review every actual browser page family, including meaningful forms, dialogs,
empty states, errors and permission-dependent variants. Compare the original
MedTracker presentation with the Rust application, then assess whether a person
arriving on each page can understand its purpose, context, main action, result
and next step. A Rails difference is not automatically a usability defect.

The source baseline is `d3dcc0f334b08ba728ba25523dbded63bd15639b`.
The inventory follows every browser registration in `src/app.rs`: 23
registrations, 159 route declarations, including 101 with GET handlers. These
are not 101 distinct pages: aliases, retired endpoints, authentication callbacks,
JSON helpers, private image responses and PDF downloads are recorded separately.
POST-rendered wizards, edit forms and confirmation dialogs are classified separately.
The complete declaration inventory is in [route-coverage.csv](route-coverage.csv).
The 26 UI families below are a review taxonomy; the ledger retains the 22
controller-family groups from the route registrations.

The audit used owned synthetic fixtures only. The retained Rails screenshots, templates and
assets are comparison sources. Retained Rust screenshots are historical context,
not evidence of current rendering. The running revision, fixture state, viewport, theme and URL accompany captures;
independent family reviewers inspected saved images before accepting evidence.

The two review lenses are:

- **Parity:** confirmed differences in branding, layout, content, actions and
  state presentation against an identified Rails source.
- **Usability:** concrete difficulty understanding or completing the task,
  including hierarchy, wording, discoverability, feedback and keyboard use.
  Recommendations and owner choices remain distinct from confirmed defects.

Accepted Rust account authentication, global people, multiple households,
guardian declarations and disclosure rules remain authoritative. Do not recreate
Rails sessions, credentials, schemas or obsolete person/location authority.
Required medication storage locations are existing planned domain work, not a
new visual defect. Scanner camera use on physical devices, live providers,
email delivery, full assistive-technology testing and deployment are outside
this synthetic browser audit.

## Coverage and first-arrival verdicts

Most states have 1440 × 1000 desktop and 390 × 844 mobile captures. Login also
has a 375-pixel comparison. Shared profile/menu has actual light appearance at
320 pixels; remaining evidence is mainly dark. Numbers link through the
[capture index](captures.md). Empty states do not establish populated success.

| Family | Inspected evidence and first-arrival verdict | Unverified boundary |
| --- | --- | --- |
| Public entry/login | 01–02, 120, 122–123: clear purpose/recovery; real synthetic sign-in succeeds; invalid-password alert adjacent. L1/L2 weaken hierarchy. | Physical passkey success and light login parity. |
| Signup/verification | 110–111, 115, 117, 119: coherent password/passkey choice and verification purpose. | No account creation, valid verification, invitation signup or delivered mail. 119 is confirmation presentation only. |
| Password recovery | 112–114, 116: generic request acknowledgement and return route; new-password form rendered. | Nonexistent synthetic address used; no delivered email, new password or token rejection submission. |
| MFA/account security | 59, 66–70, 118: methods, sessions and immutable key scope explicit; shared presentation needs C8–C10. | Entry forms only; no credential/key creation, revocation, TOTP enrolment/backup, closure/provider linking. 118 has no challenge session. |
| OAuth consent | Registered route and existing consent contract inspected. | No configured fixture client; current consent presentation, Allow/Deny and grant/callback remain unverified. |
| Household chooser | 03–05: clear household/Open action; actual selection succeeds; C1 dead security link. | Empty/create/multiple choices not rendered. |
| Navigation/search | 79–84, 125–126: readable light 320px drawer, real grouped six-result overlay, account menu; Escape returns focus to triggers. | 81 typed-before-results only; result failures/full keyboard traversal not covered. |
| Dashboard | 06–10: identity, routine/PRN distinction, dose and stock context clear; recording cancellation loses position (C2). | Current layout only; three saved variants have no exposed selector in inspected UI and remain unverified. |
| People | 48–51: clear list/detail and identity/support grouping; global/dependent creation semantics accepted. | No person save, dependent/guardian attestation or viewer variant. |
| Person care/health | 90–93: factual purpose, useful empty actions, synthetic illness saved/edit retained. 124 is an access-denied response for this fixture actor. | Carer content/new/edit and authority variants not rendered; no side-effect outcome/deletion. |
| Inventory/detail | 11–15: medicine/stock context and retained validation task clear. | Empty/large filtered list and deletion outcomes not covered. |
| Finder/add workflow | 26, 41–47: distinguishable paths, clear progress and successful synthetic medicine save. | 44–46 mobile only; no physical camera/external catalogue hit. |
| Stock/ordering | 20–25: compact forms/order status; received wording separates receipt from adding stock. | No final receipt/refill save or stale concurrent count. |
| Dosage options | 16–19: useful empty state, synthetic save and retained edit. | No deletion/invalid amount. Saved dose wording needs polish. |
| Treatments/schedules | 27–34: person/assignment context clear; required date/error placement/competing controls need correction. | Daily saved; taper/timing controls inspected but not every mode saved; no pause/retire. |
| Dose outcomes/history | 35–40: context and not-taken/reopen/taken effects clear; actual synthetic sequence completed. | Immutable taken history intentional; no full correction/date matrix. |
| Medicine reviews | 94: clear practitioner-conversation boundary and filters/empty queue. | No evidence pairs, history or POST edit/result; PDF unverified. |
| Reports/downloads | 108–109: summary/outcomes/export purpose; actionable reverse-date error retains dates. Table scrolls within card. | Both PDF outputs unrendered; no output parity claim. |
| Locations | 85–89: storage/stock context, compact forms, named deletion restriction. | Confirmation cancelled; no blocked/successful final deletion or empty list. |
| Profile/avatar/notifications | 71–76, 79: four tabs/disclosures, file/privacy/closure boundaries explicit. | No avatar uploaded, notification sent or data export. Empty file submit invoked native required-file validation. |
| Guardian declarations | 107: personal scope separated from care links; readable empty state. | No declaration states/withdrawal/pagination; 105 invalid trailing-slash path rejected. |
| Household administration | 95–99, 101–102: clear routes, role versus guardianship, detail/forms and retained invalid audit filter. | No access changes/bulk preview/settings save/populated audit detail. |
| Invitations | 100, 121: role/care and provider/delivery distinctions; designed invalid-link recovery. | Empty list; no send/resend/cancel/accept or mail status. |
| Care administration/recovery | 103–104: absent eligible minors and confirmation boundary explicit. | No eligible recovery detail/result or relationship change. |
| Platform/support | 52–58, 60–65: actual request, proof, separate owner approval, activation, read-only content and ending. Existing owner blocks recovery clearly. | No platform access changes/missing-owner recovery. 63 pre-readiness content only, stable support chrome unverified. |
| NHS dm+d catalogue | 106: archive limits, background continuation and refresh/empty history clear. | No upload/import or queued/running/completed/failed rows; no full catalogue job. |

## Captured login baseline

Runtime supplied by the performance owner: native debug binary at revision
`a825e06c4`, SHA-256
`20bde57334b5d85b4c925bb6c0c3036cd5660533c451466faa250d46875a7e93`.
Assets came from the owner's `bolt-ampp-memory` checkout. A complete Git
comparison of tracked `assets/`, `src/controllers/` and `tests/support/` against
the report baseline was empty. This does not assert that the whole binary is
the current main revision or that generated CSS was independently rebuilt.

The owner created disposable synthetic database `bolt_ui_audit_20261023` and
reported restricted application credentials without RLS bypass. The audit
started one sequential in-app browser session and no compiler, load generator,
worker, import or competing database. Saved screenshots were opened and
inspected by the owner and independent family reviewers; [capture metadata](../../screenshots/ui-audit-2026-10-23/captures.json)
records the URL, viewport, theme and observed headings.

The first session failed during the owner's service outage. The owner restored
PostgreSQL, storage and the same native server, then verified actual sign-in and
authenticated dashboard responses. A fresh in-app tab subsequently completed
sign-in and household selection. The outage is not an authentication-code finding.

The current audit has exercised the chooser, dashboard recording/cancellation,
inventory/detail/edit and validation, dose options, refill/adjustment/order/batch
stock entry, assignment/schedule/taper forms, not-taken/reopen/taken outcomes,
finder no-match recovery, medication setup/completion and people/detail/new/edit.
Screenshot names identify those states. Independent family reviews inspected both sizes and matching DOM/source.
Rejected captures numbered 29 show the wrong page and are explicitly rejected in the
manifest; number 30 contains the actual schedule form.

Synthetic changes made through the UI were one dose option, one order, one
routine schedule, one new medicine and a not-taken/reopened/taken dose outcome.
They belong only to the disposable UI database. Additional synthetic writes were a health event and temporary support request,
approval by a separate owner, activation and ending. No real clinical records,
live provider, outgoing invitation, real email delivery or benchmark database was used.

| Step | State and health | Evidence |
| --- | --- | --- |
| 1 | Desktop dark idle login renders without horizontal overflow; action hierarchy needs correction | [Current capture](../../screenshots/ui-audit-2026-10-23/01-login-desktop.jpg), [Rails desktop reference](../../screenshots/passkey-login-idle.png) |
| 2 | Mobile dark idle login renders without horizontal overflow; duplicate chrome delays the form and recovery links | [Current capture](../../screenshots/ui-audit-2026-10-23/02-login-mobile.jpg), [Rails mobile reference](../../screenshots/login-lighthouse-mobile.png) |
| 3 | The first sign-in attempt was blocked by a synthetic database outage; a fresh tab successfully signed in after recovery | The failed state was rejected; subsequent chooser and care captures show the recovered session |

The Rails references are light appearance; the mobile reference also contains a
passkey error. Colour and error-state differences are therefore not treated as
pixel mismatches. This comparison supports the structural findings below only.
It does not verify the separate prepared login correction.

## Non-page boundaries

- Native portable import/export APIs, MCP, OAuth token/revocation and care APIs
  have no separate browser page in this route inventory. Their protocol and
  authority testing is outside this visual audit. This is not a finding that
  an import screen ought to exist.
- NHS dm+d archive import is an actual browser page and is included. Inspect
  synthetic progress/error records without launching an overnight catalogue job.
- Profile history export is a protected POST download; health-history and review
  reports are protected GET PDFs. Inspect the controls and downloaded output
  separately from the page that links to them.
- Avatar GETs return private image bytes. The upload POST can re-render profile
  validation/storage errors and therefore needs an interactive state capture.
- Provider-link confirmation is an actual page when configured. The provider
  completion route can also show a one-time personal-key result. Lack of a live
  external provider must be recorded as a limit, not substituted with a mock
  success claim.
- Retired Rodauth/WebAuthn routes intentionally return Gone. They need no
  independent visual redesign; active pages must not send users to them.

### L1: Sign-in actions lose their button affordance in dark appearance

**Priority: high. Type: confirmed usability issue and parity gap in the captured
baseline.** Both current captures show Sign in and Sign in with a passkey as
text against the form panel, with no clear filled or outlined boundary. The main
action does not stand out from its alternative. Rails presents a filled teal
primary button and a framed passkey button. A user must infer which text submits
the form and which opens an alternative sign-in flow.

Owning sources: `assets/views/auth/login.html`, the login rules in
`assets/static/css/app.css` and shared button/theme tokens. Verify generated CSS
against source before attributing this to a particular token defect. The smallest
correction is to give the existing primary and alternative controls distinct
fills/borders, preserving their native actions, labels, loading and focus states.
The separate prepared login correction already addresses this presentation; judge
its actual dark desktop/mobile captures before accepting the fix.

### L2: Duplicate mobile chrome pushes sign-in and recovery down

**Priority: medium. Type: confirmed usability issue and parity gap in the
captured baseline.** The 375-pixel capture shows the MedTracker mark in the
global header and again inside the login card. The header wraps to two rows;
the form begins around 450 pixels down and recovery reaches the bottom edge of
the first viewport. The Rails reference has one visible brand and begins the
form around 350 pixels down. Scrolling remains possible, but secondary sign-in,
recovery and account-creation options are harder to discover on arrival.

Owning sources: `assets/views/auth/login.html`, `assets/views/layouts/base.html`
and scoped login spacing in `assets/static/css/app.css`. The smallest correction
is a login-specific frame with one brand and retained Appearance access. Do not
alter signed-in household navigation. This is also covered by the separate
prepared login correction, whose actual rendering remains pending.

First-arrival verdict: the welcome text and two-panel structure explain the
purpose, and the illustration adapts without horizontal overflow. The primary
action needs more visual weight; mobile needs a shorter route to the form.

## Further confirmed findings

| ID / priority | Observed difficulty and smallest useful correction | Evidence and owning source |
| --- | --- | --- |
| C1 / high | The chooser’s Security settings link leaves the designed UI for raw retired-route text. Point the existing link to `/account/security`, then check keyboard/touch navigation. | Captures [03](captures.md#step-03), [04](captures.md#step-04), [05](captures.md#step-05); `assets/views/home/index.html:9`, `src/controllers/auth.rs:68` |
| C2 / medium | Dashboard Take opens a new page with an automatically opened dialog. Escape closes it but focus falls to the page body; the dose query also remains in the URL. Restore focus to the matching Take link and clear only recording query keys, preserving selected person/grouping. | Captures [09](captures.md#step-09), [10](captures.md#step-10); same-viewport repeated keyboard observation; `assets/static/js/interaction.js:86`, `assets/views/dashboard/task.html:4`, `assets/views/dashboard/next.html:5`. Existing Level A test covers a same-page medication button, not this navigation. |
| C3 / medium | The schedule form leaves End date blank without saying it is required. Saving a daily schedule produces “Invalid operation” plus “must be a date”; adding a date succeeds. Identify required dates before submission and return an actionable summary. Preserve the existing clinical requirement. | Captures [30](captures.md#step-30), [32](captures.md#step-32), [33](captures.md#step-33); `assets/views/treatments/field.html:7`, `src/controllers/treatments/forms.rs:210`. Rails marked End date required in `rails/app/components/schedules/fields.rb:349`, so this is also a confirmed interaction parity gap. |
| C4 / medium | The refill dialog does not identify the medicine or the quantity unit. Include both beside Quantity to add. | Capture [20](captures.md#step-20); `assets/views/medications/show.html:80`. The separate adjustment page already provides this context. |
| C5 / medium | A barcode miss appears below the separate catalogue-search form, far from Find barcode. Put the result and manual-entry recovery beside the action that produced it. | Capture [42](captures.md#step-42); `assets/views/medications/finder.html:23` |
| C6 / medium | New Person defaults to Minor and says minors/dependent adults have no capacity, while the capacity checkbox remains available. Source validation rejects a dependent with capacity; that invalid combination was not submitted in this audit. Make availability and helper text match the selected type. This does not change dependent-only creation or guardian authority. | Capture [51](captures.md#step-51); `assets/views/people/form.html:22`, `src/models/care/people/creation.rs:77` |
| C7 / medium | Schedule field feedback is misplaced in the desktop grid: the End date error appears under Start date, and limit help sits beside its field. Keep help and errors inside the field wrapper. | Captures [32](captures.md#step-32) desktop and [34](captures.md#step-34) desktop; `assets/views/treatments/field.html:8`. The partial closes its wrapper before emitting feedback. |

These are bounded follow-ups, not application changes in this report branch.
The prepared login fix covers L1/L2 separately. Dashboard C2 belongs to the
already approved subsequent dashboard slice; no implementation is included here.

## Usability recommendations from inspected care screens

| Recommendation | User impact and evidence | Small correction |
| --- | --- | --- |
| Explain the two medication-add paths | Inventory shows Add Medication and Add medication for someone without explaining inventory creation versus assigning care. Captures [11](captures.md#step-11), [12](captures.md#step-12); `assets/views/medications/index.html:10`. | Use task-specific labels or brief helper text. |
| Remove implementation wording from stock actions | Medication detail offers Adjust stock and Stock adjustment page for the same task. Capture [13](captures.md#step-13); `assets/views/medications/show.html:71`. | Prefer one entry point or describe the meaningful alternative. |
| Make field errors visible beside their field | Invalid Name retains the ordinary input appearance; the error is only in the summary above. Capture [15](captures.md#step-15); `assets/views/medications/input.html:6`. Programmatic invalid/description wiring exists; this is not a claimed WCAG failure. | Add a visible local error/state using the existing validation data. |
| Separate destructive from routine management actions | Delete Medication uses the same ghost treatment as Edit and stock actions. Confirmation exists. Capture [13](captures.md#step-13); `assets/views/medications/show.html:74`. | Separate its placement or destructive styling. |
| Polish dose display | Saved options show “2.0 tablet”. Capture [18](captures.md#step-18); `assets/views/dosage_options/index.html:11`. | Format integral amounts and pluralise display units. |
| Reveal relevant schedule timing controls | Daily schedules display specific dates, weekday choices and taper controls together. Captures [30](captures.md#step-30), [31](captures.md#step-31), [34](captures.md#step-34); `assets/views/treatments/form.html:20`. | Show the controls needed for the selected schedule type and explain each input locally. This is a usability recommendation, not a proven Rails styling mismatch. |

## Source observations and limits

### Additional bounded findings

| ID / priority | User impact and correction | Evidence/source |
| --- | --- | --- |
| C8 / medium | Security entry forms have weak primary actions against dark cards, while the API-key form has a clear filled action. Give the existing shared primary class a consistent visible treatment; retain native actions and focus states. | [53](captures.md#step-53), [66](captures.md#step-66), [67](captures.md#step-67), [69](captures.md#step-69), [70](captures.md#step-70) versus [68](captures.md#step-68); `assets/views/identity_onboarding/password.html`, `totp.html`, `add_passkey.html`. Source has no `shell-primary` rule; generated CSS has not been independently rebuilt. |
| C9 / medium | At desktop size, labels and inputs in security forms run beside one another and wrap into confusing groups. Stack each label and its full-width field consistently. Programmatic names exist; this is a visual grouping finding. | Desktop [53](captures.md#step-53), [67](captures.md#step-67), [69](captures.md#step-69), [70](captures.md#step-70); the same identity-onboarding templates. |
| C10 / medium | The support proof screen has the correct heading but the document title says Change password and its action says Confirm change. Use the operation name in the title and confirmation button. | [53](captures.md#step-53); `src/controllers/identity_onboarding/password.rs:63`, `assets/views/identity_onboarding/password.html:21`. |
| C11 / medium | The profile Security summary clips the final passkey status on mobile. Wrap or stack all three status items within the available width. | [72](captures.md#step-72) mobile; `assets/views/profile/security.html:2`; retained Rails security mobile reference. |

Lower-priority improvements: distinguish saved reminder preferences from delivery
availability ([73](captures.md#step-73)); use plain support statuses and an adjacent next step after
confirmation ([54](captures.md#step-54), [64](captures.md#step-64)); give TOTP/passkey entry forms a local return link
([69](captures.md#step-69), [70](captures.md#step-70)). The shared header already provides Account security, so these are
not trapped routes. The expanded avatar heading/help repeats its disclosure
summary ([75](captures.md#step-75)), which is optional polish rather than a parity defect.

The PDF browser download did not return. No downloaded first page was inspected.
This remains an evidence limit, not an application defect.

1. **Login fidelity:** #2895 reused the medication illustration but retained
   different branding, spacing and controls. A separate correction is prepared
   on `codex/fix-login-reference-layout`; it needs fresh comparison and checks.
   It is not included in this report branch.


### Report interpretation and recovery findings

| ID / priority | User impact and bounded correction | Evidence/source |
| --- | --- | --- |
| C12 / medium | Daily Compliance and Daily Doses Logged actually aggregate the selected seven-day period. Rename them for the selected period, preserving calculations. Rails has the same labels, so this is inherited ambiguity. | [108](captures.md#step-108); `src/models/care/reports.rs:1193`, `assets/reports/locales/en.yml:2011` |
| C13 / medium | Days with zero expected/zero taken have full 100% bars identical to a completed 1/1 day. Distinguish No doses expected visually and in the chart description, without changing arithmetic. | [108](captures.md#step-108); `src/models/care/reports.rs:1091`, `assets/views/reports/index.html:29` |
| C14 / low | Missing-token reset/verification URLs show raw query-deserialisation text with no recovery link. Render a branded explanation with reset/resend routes, following the designed invitation error. This is malformed-entry recovery, not a token-validation security finding. | [114](captures.md#step-114), [117](captures.md#step-117) versus [121](captures.md#step-121); `src/controllers/identity_onboarding/password_reset.rs:63`, `confirmation.rs:30` |

The mobile outcomes table is 511 pixels wide inside a 316-pixel
`overflow-x: auto` container; it does not cause document overflow. Smaller
improvements: add days to the report inventory estimate ([108](captures.md#step-108)), distinguish
an unfiltered empty audit from no filter matches ([101](captures.md#step-101)), avoid repeating an
email fallback name beside the identical email ([96](captures.md#step-96)), and reserve No results
for a submitted search ([82](captures.md#step-82) versus [84](captures.md#step-84)). The invitation error says Return to
dashboard but links to public home for signed-out users ([121](captures.md#step-121)–[122](captures.md#step-122)); use a
destination-accurate label. Person-carer denial ([124](captures.md#step-124)) is permission-state
evidence only, not an authorised-content or permission-defect claim.

## Verification, review and delivery

- Every registered route is mapped to a page/state, alias, response helper,
  retired route, download, or explicit untested boundary.
- Every page family has inspected current desktop/mobile screenshots or a named
  blocker, plus a concise first-arrival usability verdict.
- Findings name the screen/state, user impact, priority, evidence, source location
  and smallest useful correction. Shared causes are grouped.
- Keyboard, focus, labels, error recovery and reflow checks are reported with
  their actual scope. Automated checks do not establish full WCAG conformance.
- An independent reviewer checks coverage and the evidence behind findings.
- The report must pass documentation checks and be published separately from
  application fixes. No merge or deployment is authorised by this audit.

Three read-only reviewers inspected care/report, administration/profile and
authentication/accessibility families, including current desktop/mobile images,
DOM and relevant source/Rails references. They rejected the transient support
sidebar as a persistent defect and corrected initial-form and token-state claims.
Two independent completion reviewers approved the corrected report/ledger.
They checked route matching, method claims, rejection boundaries and screenshot
index formatting. Documentation build and diff checks passed; see the
[verification record](verification.md).

No full WCAG conformance, live mail/provider delivery, physical-device acceptance
or production deployment is claimed. Keyboard evidence is limited to recorded
dialog/menu/search Escape and focus observations; contrast ratios and full
assistive-technology use were not measured. The PDF download stalled and produced
no reviewable first page. The existing login correction remains separate and
requires its own application verification.
