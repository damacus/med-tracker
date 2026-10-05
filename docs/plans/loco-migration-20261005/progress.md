# Migration progress

Updated 5 October 2026. Guarded adoption, shared tenant access, runtime queue
provisioning and the first Take domain operation pass their focused checks.
Devin accepted the combined repair batch. The final source is frozen for full
Loco CI and publication.
No complete migration section is accepted yet.

| Area | Verified current state | Remaining acceptance work |
| --- | --- | --- |
| Loco foundation and Rails relocation | Published in draft PR #2451; full local checks and review passed | Corrected hosted run must pass with full Git history |
| Schema and tenant persistence | Persistence 16/16; tenant access 11/11; exact baseline and metadata preserved; restricted-role server/worker boot passes | Final review, full CI, normal seed/task integration and populated Rails rollback |
| Sign-in and API authentication | Isolated maintained-library proof: 39/41, with two retained wrapper diagnostics | Durable stored-format/concurrency checks and full browser/OAuth/SMART integration |
| Care operations | Take 20/20: atomic stock/audit, replay, timing and account-zone selection, tracked stock, member scoping, concurrency, conflict, revoked-access denial and audit rollback | Real authenticated Loco routes and every remaining care operation |
| APIs and integrations | Existing contracts and migration inputs retained | Migrate complete native/FHIR/SMART/MCP journeys |
| Browser pages and themes | Foundation Tera page works | Complete browser workflows and port themes to daisyUI |
| Background jobs and reminders | Owner-provisioned PostgreSQL queue supports restricted Loco startup | Actual job execution, schedules and reminder migration |
| Release and cleanup | Useful source/PR inputs protected; Rails remains independently runnable | Both scratch architectures, complete journeys, populated rollback and obsolete tooling removal |

## Current delivery

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
head 705e1f09. The current product change remains local until review and full CI
pass. The user authorised landing verified work on main through one branch/PR;
no production deployment or database mutation is authorised.

Hosted run 37371230690 initially lost a household runner. Its failed job was
retried and passed. Loco then reached its inventory check and failed because
the shallow checkout lacked the retained baseline Git commit. The corrected
Loco checkout fetches full history. Its failing regression is now green: six
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

The next implementation boundaries are durable identity storage and stock
adjustment using existing SeaORM logic. Current source stays frozen for review and
publication; preparation during that interval is read-only. Defects within the
migration stay in their owning work rather than becoming follow-up issues.

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
checkpoint. The next two-hour retro is due at or after 22:49 UTC.
