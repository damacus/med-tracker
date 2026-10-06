# Migration progress

Updated 6 October 2026. No complete migration section is accepted yet.
The full migration remains the active goal. The eight areas are a coverage
checklist, not eight serial planning stops.

| Area | Verified current state | Remaining acceptance work |
| --- | --- | --- |
| Loco foundation and Rails relocation | Published in draft PR #2451; local checks and hosted Loco job pass | Complete application and retained rollback verification |
| Schema and tenant persistence | Local persistence 20/20 and tenant access 11/11; additive adoption preserves the baseline; normal seed CLI passes | Populated Rails rollback and full schema acceptance |
| Sign-in and API authentication | Identity 23/23 and bearer validation 5/5; encrypted browser sessions survive a real process restart | Selective browser middleware, final review, all retained credentials and SMART integration |
| Care operations | Local Take 28/28 and stock operations 19/19; UUID, skipped-midnight and removal/API independently reviewed | Every remaining care operation and user journey |
| APIs and integrations | Ten real Loco listener tests pass, including stock removal/history, authentication challenges, authorization ordering and session-enabled cookie-free bearer responses | Complete native/FHIR/SMART/MCP contracts and strict HTTP idempotency |
| Browser pages and themes | Actual care journeys and shell checks pass desktop/mobile, including navigation, Origin/CSRF, restart, expiry, revocation and request IDs | Final browser/session review, remaining workflows and offline/PWA |
| Background jobs and reminders | Owner-provisioned PostgreSQL queue supports restricted Loco startup | Actual job execution, schedules and reminder migration |
| Release and cleanup | Useful source/PR inputs protected; Rails remains independently runnable | Both scratch architectures, complete journeys, populated rollback and obsolete tooling removal |

## Current local work

Persistence passes 20 tests and doses pass 28. Five UUID regressions and the
skipped-midnight regression failed before their repairs. UUID variants now
share one lock and unique identity, historical stored spelling is preserved,
and duplicate identities across households refuse adoption without rewriting
records. The maintained timezone library resolves skipped midnight to the
first valid local boundary. Devin SWE-2 Max approved these bounded repairs in
`spiritual-titanium`; this does not approve identity, the HTTP/browser facade
or the whole migration.

Ten real API tests pass, including clinical-audit rollback with one retained
HTTP-attempt audit. Header, response and audit share Loco's request ID. Hidden
UUID conflicts preserve a private code without disclosing another household's
record. Formatting and warnings-denied lint pass. Authentication challenge
headers and insufficient-scope classification now pass their regressions.

The OAuth review's suspected confidential refresh bypass was disproved by
unchanged-production tests for both secret-post and Basic clients. Pinned library
source confirms recovered grants still invoke client authentication. Keep that
regression. Invalid stored PKCE error classification, narrowed access scope with
preserved refresh authority and stored Rails timezone names now pass focused
checks. Devin accepted these bounded repairs in `shore-pincushion` and accepted
the bounded stock-removal/API implementation in `hurricane-brazil`. Neither
verdict approves complete identity, browser integration or the whole migration.
The confidential-secret negative test passed without a production change; the
reviewer's reference to a RED-to-GREEN repair for that finding is incorrect.

The extended desktop run proves clinical effects, actual process restart,
encrypted session persistence, revocation, expiry and CSRF-token rejection.
It exposed three defects: browser audit request IDs differ from response headers,
foreign-origin forms are accepted and stale-stock errors lack a recovery link.
These repairs and real scoped sign-in navigation pass the complete
desktop/mobile journey. Session-enabled API tests now prove zero
bearer-response cookies with middleware applied only to explicit browser routes.
The duplicate homepage HTTP runner is removed after its health/static coverage
passed through the actual browser journey. Normal `task db:seed` now passes its
real CLI test with standard Loco fixtures, usable credentials and a household
medication. Explicit command environment forwarding ensures the supplied seed
database overrides a conflicting ambient database URL. Synthetic seeds remain
limited to empty development/test databases.

Full repaired candidate `task ci` passes (10199), including API10, dose28, stock19,
identity23, resource5, persistence20, tenant11 and all fourteen desktop/mobile
browser cases. Owned test resources are gone. Unused public demo tabs and the
duplicate foundation HTTP runner are removed; real appearance controls and
health/static/header checks remain in the application browser suite.

Final browser review identified a missing dose form for a second permitted
person and a Linux-incompatible test-output path. The real two-person browser
test failed with one form where two are required (70688), before production
changes. Both repairs pass real desktop/mobile tests (39164). Devin SWE-2 Max
accepted the browser/session/seed integration corrections in `keen-sunfish`;
its conditional full-CI requirement is satisfied by 10199. The review's other three requirements
are already enforced by model-level stock authorization, eligible-source
filtering and active household lifecycle checks; their existing tests pass.

These product changes are published at `3840bded`, including the clean merge of
main's dependency update `e8838fb4`. Classifier repair `60a70d4d` and Markdown
repair `42636dec` are also pushed. Exact-head hosted run `37398111231` passes
Loco, classification, Markdown, documentation and workflow checks. The overall
run is still active; its Rails browser shard fails the admin membership-role
filter because the listbox does not open. This is a different failure from the
older scanned-stock notice. Neither result proves complete rollback acceptance.

## Medication management and OAuth delivery

Medication create, partial update and guarded deletion pass all 29 domain tests.
Three real HTTP create/update cases pass, including stale ETags, validation and
permissions. A real browser test then failed on the missing Add Medication link;
the create, edit and guarded-delete pages now pass desktop/mobile verification.
Independent review identified a blank-dose edit that can change dose mode;
its scheduled-medication browser regression failed before the correction and
now passes on desktop and mobile, alongside specific Location and Unit errors.
Three extra domain cases pass for empty barcodes, oversized invalid doses and
intentional nullable API updates. The next location tests remain unregistered during this delivery's
verification, so they do not change its application build.

The corrected OAuth HTTP suite passes all twelve cases in run `7196`. Actual
browser tests previously exposed excessive code lifetime, missing transactional
consent/revocation audits, permissions granted after being declined and a
discovery issuer missing the configured port. The corrections pass actual
desktop/mobile journeys, including retained query and form_post callbacks.
Independent review found no proven release blocker in the supplied diff.
Refresh consent, audit attribution and membership findings were checked against
the retained application. The hidden Stay signed in choice reached a real
browser failure, then passed on desktop and mobile after its correction.
Both correction reviews approve the changes.
These results do not accept complete identity or SMART integration.

Queue run `31436` passes three real PostgreSQL cases with the restricted runtime
role. It proves setup without runtime DDL, provider reconnection without lost
queue records, maintained failed-job retry and stale-job requeue, and actual
automatic recovery by Loco's reaper followed by bounded normal shutdown.
Automatic recovery was disabled before the failing configuration test; standard
development, test and production configurations now enable Loco's reaper.
Provider reconnection does not prove application process restart. Real worker
delivery, duplicate side effects, schedules and process crash/drain remain required.
No queue adapter, fake application worker or extra runtime runner was added.

Final full local `task ci` run `25075` passes, including all 40 desktop/mobile
browser cases, 29 medication tests and the complete root Rust checks.
Owned test containers, volumes and networks are removed. An earlier full run
passed all Rust checks and 39 browser cases but sampled a button during a
200-millisecond colour transition. The existing measurement helper now waits
for actual CSS transitions to finish, bounded to five seconds; colours and
contrast thresholds are unchanged. Focused desktop/mobile verification and
the final full run pass. Publication follows this verified candidate.

The remaining one-time Rails relocation structure audit now runs through
`task migration:audit`, alongside other historical migration audits, instead
of daily CI. Its regression failed before the Taskfile change and passes all
eight cases afterwards. Database isolation, cleanup and application tests
remain in daily CI. No new runner or implementation handoff document was added.
The worker plan uses the first real application job to prove delivery instead
of adding a temporary probe worker.

## Earlier verified delivery

The focused checks pass on the final frozen source: persistence 16/16 plus one
explicit capture-only test ignored; tenant access 11/11; care 20/20; runner 9/9;
database helpers 4/4. Formatting and Clippy pass. Every owned PostgreSQL fixture
was removed after its run; pre-existing volumes were preserved.

The queue capture adds exactly 19 named objects. Shared access rechecks account
lockout, current membership role/version and live person grants in the same
transaction. Denied source view returns NotFound, matching retained privacy
behaviour; no clinical effects remain after denial or failed audit.

Devin passed the earlier adoption review. Its three bounded corrections are now
implemented: explicit 177-row/nonempty Rails metadata assertions, safe system
schema lookup during catalog checks, and refusal of caller-owned transaction
adoption before any role/settings changes. The combined review approved its bounded
scope. Its repair batch now passes focused checks: precise duplicate conflict
handling, tracked-stock timestamps and explicit request-zone selection. Canonical
account attribution is preserved and covered across households. Devin accepted
the repair batch. Its requested member-role green-path coverage passes; coordinator
review confirms linked stock succeeds and unlinked stock stays excluded. Production
source is unchanged from the reviewed freeze. Full CI passed before that final
coverage-only addition and is repeated on the final source before publication.
This verifies domain behaviour, not HTTP routes or completed application identity.

## Published and hosted state

[Draft PR #2451](https://github.com/damacus/med-tracker/pull/2451) has published
head `0c1a70dcd325134af4ef23cc9ea643834beb1972`. Guarded persistence, tenant
access and the first Take were published earlier. The latest commit removes
five historical audits from daily CI; `task migration:audit` retains them.
Their full-history checkout and ripgrep installation are also removed.
Hosted run 37386310209 finished with Loco Foundation passing. Overall CI failed
on the existing Rails scanned-stock notice assertion at
`rails/spec/system/medications/refill_inventory_spec.rb:77`. This prevents
claiming full rollback verification. Nothing is merged or deployed.

The user authorised landing verified work on main through one branch/PR;
no production deployment or database mutation is authorised.

Earlier hosted run 37371230690 initially lost a household runner. Its failed job was
retried and passed. Loco then reached its inventory check and failed because
the shallow checkout lacked the retained baseline Git commit. The corrected
Loco checkout fetched full history at that stage. Its regression passed: six
focused checks, 94 CI script tests and workflow lint pass. Coordinator source
review passed. This routine prerequisite fix will publish with the capability.

## Workflow after the retrospective

The eight areas are a coverage checklist, not eight serial implementation stops.
Locally verified prerequisites permit development while hosted checks run. Astra
owns shared persistence/identity; Nightingale owns disjoint care operations. Browser
preparation has mapped all ten selectable palettes and retained saved monochrome
tokens; its disjoint shell implementation can start after this publication freeze.
The verifier alone owns builds and disposable resources. Root owns integration
and publication. Devin reviews coherent capabilities in fresh source-only sessions;
routine tooling fixes receive focused tests and coordinator review.

The next usable delivery is persistent sign-in with dose and stock forms.
Nightingale is also porting complete stock removal from its retained contract.
Reviewed API snapshots stay fixed while new disjoint modules progress. Defects
within the migration stay in their owning work rather than becoming follow-up
issues. Devin receives actual diffs under the user's approval through
16 October 2026. Automatic approval review rejected a broader complete-source
packet; the accepted replacement is diff-only. No permission bypass was used.

## Retained scope and evidence

[Issue #2450](https://github.com/damacus/med-tracker/issues/2450) covers the entire
migration. Superseded PRs #2397, #2399, #2402 and #2403 are closed with inputs
protected. Schema, scratch, profile, notification and font work stays available
until replacements land. PR #2381 is unchanged. Incident #2453 remains separate.

Earlier foundation/Rails evidence is in foundation-report.md. Legacy RSpec
diagnostics are stopped; original Rails code/tests remain the rollback reference.
No source-only experiment or old green run proves completion of the new app.

The user reports are /private/tmp/medtracker-migration-status.html and
/private/tmp/medtracker-2450-review-status.md. Update at a completed slice or when
a problem changes the next action. Detailed handles stay in the technical
checkpoint. The next two-hour retro is due at or after 00:54 UTC on 6 October.
