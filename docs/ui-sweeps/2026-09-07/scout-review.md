# Independent scout review

Reviewed against baseline `45db2112`. This review does not treat source evidence as a visual
defect. INV-03 and INV-04 were reproduced locally by the coordinator on 7 September; ADM-05
was reproduced as an authorised dashboard page linking to an authorisation redirect.
No tests, server, browser, or Canary work was run by this reviewer.

## Requirements verdict

| Candidate | Verdict | Evidence and required acceptance check |
| --- | --- | --- |
| INV-01 location count | Accept as a wording defect | `IndexView` derives the noun from the first word of the success flash, while `medications.created` is a complete sentence in every supported locale. In Irish the rendered noun is `Cruthaíodh`, so a one-item badge is grammatically wrong. Add a dedicated count key with `one` and `other` forms in all five locale files. Render one and two medications under `en`, `cy`, `es`, `ga`, and `pt`; assert the badge matches the locale count translation rather than any creation-flash wording. |
| INV-02 low-stock badge | Accept as a wording defect | `ShowView#render_stock_badges` renders the literal `Low Stock` when `low_stock?` is true. Add a location stock-warning key in every supported locale and cover a low-stock medication in a locale-aware component example. The browser check must visit a low-stock location under each locale and confirm that no English `Low Stock` remains outside English. |
| INV-03 invisible member removal | Accept as P1 accessibility defect | The trigger has `opacity-0 group-hover:opacity-100`, with no focus or touch-visible state. The coordinator reproduced it locally at 390px dark mode: after `Add member`, Tab reaches the `Remove member` button and its computed opacity is `0`. Make the labelled control visible without hover; an always-visible icon is the smallest consistent fix. Browser regression: at 390px dark mode, Tab from `Add member`; assert the active remove button has opacity `1`, an in-viewport non-zero rectangle, and an accessible name. Open it, cancel, and assert focus returns to the visible trigger. |
| INV-04 profile tabs | Accept as P2 layout/accessibility defect | The triggers have `min-w-0` and the list has `h-auto`, but the coordinator reproduced overflow locally at 320px dark mode: `Notifications` has a 69px client width and 72px scroll width, and its text overlaps adjacent labels. Use a deliberate narrow-width label strategy while retaining the RubyUI tabs semantics. Regression: at 320px, in long translations and at the agreed enlarged-text setting, every tab must have an in-viewport non-zero rectangle, contained text, no page overflow, and working keyboard activation. |
| INV-05 finder result metadata | Keep as a bounded reproduction candidate | The card's left column already has `flex-1 min-w-0`; the report incorrectly says it does not. The right badge column is still `shrink-0` and its text has no break rule, so long externally supplied badge labels can overflow or starve the title. Stub a result with realistic longest known labels and, separately, a deliberately unbroken label at the accepted upstream limit. At 320px and 390px, assert page overflow is at most 1px, every badge rectangle stays inside its card, and the visible title is not displaced outside the card. Do not add a fix unless this reproduces. |
| INV-06 finder package-size label | Accept as a wording defect | The result card emits literal `Pack size:` even though `FinderView` already supplies the translated `detailsPackage` value used by the details panel. Reuse that supplied translation for the summary label. Browser regression: stub `package_size`, run the search in each supported locale, and assert the summary label equals the locale package translation. The existing browser fixture already supplies `package_size`; extend that scenario instead of creating a second transport stub. |
| ADM-01 admin users labels | Accept as a wording defect | `Admin::Users::IndexView` emits literal `User Management` and `New User`; every locale only has `admin.users.index.helper`. Add `title` and `new_user` keys to every locale and render them. Browser check: under `pt` and `cy`, the heading and new-user link must match their translations and the existing helper must stay translated. |
| ADM-02 invitation roles and labels | Accept as a wording defect | The invitation form and recent rows call `titleize`; the translated form tree only supplies `email`, `role`, `dependents`, `dependents_hint`, and `submit`. The English fallback defaults therefore display outside English. Add explicit keys for the relationship and access labels, relationship placeholder, membership roles, relationship types, access levels, and the role shown in recent rows. Render through I18n keys, not `titleize`. Browser check: under `pt` and `cy`, inspect every select option and a pending-invitation row; no generated English enum text or English default label may remain. |
| ADM-03 mobile rail labels | Keep as a reproduction candidate, with source correction | The link already has `min-w-0`; the source report is wrong on that point. A fixed 2.5em label box can still clip a three-line label, particularly for Portuguese or Welsh Finder. Use three saved shortcuts including Finder, Medicine reviews, and Administration on an administrator account. At 320px and 390px in `en`, `pt`, and `cy`, assert each label is contained by its link and rail, has `scrollHeight <= clientHeight`, and remains readable when focused. Accept only on a failed geometry check. |
| ADM-04 invitation email row | Keep as a reproduction candidate, with source correction | At 320px the row is `flex-col`, so its `shrink-0` actions do not compete horizontally with the email. A legal unbroken local part can still overflow the plain email paragraph. Create a pending invitation with a valid 64-character local part and a normal domain; at 320px and 390px assert page overflow is at most 1px, the email is contained in the card, and both actions remain visible and operable. Use `break-words` only if that check fails. |

## Scope corrections

- The UI plan calls for browser confirmation before accepting visual candidates. INV-05, ADM-03, and ADM-04 therefore stay out of the first implementation tranche unless their stated checks fail.
- Locale selection is exercised in component/request tests with `I18n.with_locale`; no web-facing locale switch was found. Browser scenarios need the harness's established locale setup before they can claim non-English results.
- The inventory report's exact route and existing-test inventory are useful coverage notes, but route omissions are not defects and should not enter the defect ledger.

## Code-quality verdict for an implementation diff

No product diff was available for review. A suitable small diff should preserve the current Phlex and translation structure, add no database or authorisation change, and place regression coverage beside the affected component or existing browser scenario. In particular, it must not derive UI nouns from flash messages, use `titleize` for translated enum values, or introduce an unbounded layout workaround for candidates that have not reproduced.

For INV-03, the required quality bar is a visible, labelled control with retained dialog focus behaviour. The change should not modify membership policy or deletion behaviour.

## ADM-05 — household owners are offered a platform-only dm+d import

**Requirements verdict: accept as P1 misleading-action defect.** The local reproduction is
consistent with the source: `AdminDashboardPolicy#index?` permits household managers, but
`AdminNhsDmdImportPolicy#new?` permits only active platform administrators. Despite that,
`Admin::DashboardMetricsQuery` always creates the missing, failed, stalled, and stale dm+d
attention links, and `Components::Admin::Dashboard::IndexView` always adds the operations link.
Both targets are `new_admin_nhs_dmd_import_path`, so a household owner reaches the authorisation
redirect after being told to act.

The narrow fix is to use the existing `AdminNhsDmdImportPolicy` result when building the dashboard
capabilities. Pass that capability from `Admin::DashboardController` into
`Admin::DashboardMetricsQuery`; omit dm+d attention items when it is false, and pass the same
capability to the dashboard component so it omits `import_dmd`. Keep the controller policy and
the import controller unchanged. This preserves the platform-only boundary and prevents the
status badge from reporting a non-actionable issue.

Add a red request regression before implementation:

1. Sign in as a household owner without a `PlatformAdmin` record and ensure no completed dm+d
   import exists.
2. `GET admin_root_path` succeeds but has neither `new_admin_nhs_dmd_import_path` nor the dm+d
   attention title/action. Its status must not count the dm+d item.
3. Grant active platform administration to the same account. The dashboard again exposes both
   the operations action and the missing-import attention link.
4. Retain the existing request assertion that a household manager receives an authorisation
   redirect on a direct import URL; the visibility change is not an authorisation change.

**Code-quality verdict:** do not hide only the link in the view. That leaves `Needs attention`
with an item the person cannot act on. Do not weaken `AdminNhsDmdImportPolicy` or infer authority
from the household role. A typed capability passed at the controller/query/component boundary is
the smallest coherent change and keeps the global-catalog privilege check in one policy.

## AUTH-01 — Rodauth pages repeat the same visible flash

**Requirements verdict: accept as P2 duplicate-feedback defect.** On sign-out, the local login
page has two visible alerts with the same `You have been logged out` text: the application layout
renders `Components::Layouts::Flash` in `#notice-stack`, and the login card renders the same
`flash_message` in `#login-flash`. Both are RubyUI alerts, so this is neither a hidden
screen-reader fallback nor a deliberate live-region mirror. The resend and reset-password-request
pages use the same layout-plus-inline-flash shape, and errors follow the same path.

Keep the inline form feedback and suppress the application-layout flash when the Rodauth page
owns the rendered flash. Do not change session, redirect, error, or flash-setting behaviour. Add
browser coverage for a sign-out then login response and an invalid-login response: each must have
exactly one matching visible alert, it must be in the form/card's flash region, and the global
`#notice-stack` must not repeat it. Exercise one secondary Rodauth form such as reset-password
request to prevent a login-only special case.

**Code-quality verdict:** centralise the layout decision around the existing `content_for(:flash)`
contract or a narrowly named auth-layout hook. Do not delete inline alerts or rely on CSS/ARIA to
hide a second rendered message; either approach leaves feedback location or accessibility
semantics fragile.

## AUTH-02 — anonymous auth copy exposes Rodauth terminology and a repeated heading

**Requirements verdict: accept as a copy-only defect.** `VerifyAccountResend` renders the
locale value `Resend Verify Account Information`. It and `ResetPasswordRequest` compose the
prefix `Back to Login` with `sessions.login.heading`, whose English value is `Welcome back`; the
result reads `Back to Login Welcome back` even though only the heading is linked. The local
signed-out resend page confirms this source path.

Replace the resend action with the direct, user-facing equivalent `Resend verification email` and
render one link labelled `Back to sign in` in both shared footer patterns. Synchronise the amended
keys in all supported locale files. Do not alter paths, form names, request methods, Turbo
settings, Rodauth configuration, or auth behaviour. Add narrow rendering/browser checks for the
resend and reset-password-request pages: their submit and login link use the new labels and the
footer no longer includes the login heading as concatenated navigation copy.

**Code-quality verdict:** use one explicit navigation-copy key rather than joining a prose prefix
to an unrelated page-heading key. Keep the change in translations and view copy; it needs no
authentication refactor.
