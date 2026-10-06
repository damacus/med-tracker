# Migration progress

Updated 6 October 2026. Complete every slice in [the plan](plan.md).
No complete migration area is accepted. Nothing is merged or deployed.

## Published

The current published delivery at `4423ecf9756869dd7993f733823af4ce6902af61` is pushed to
[draft PR #2451](https://github.com/damacus/med-tracker/pull/2451).
Loco owns root commands, routes, configuration and templates. Rails is independently
runnable under `rails/` as the reference and rollback application.

Published workflows include persistent browser sign-in, permitted household
medication pages, medication create/edit/guarded deletion, dose recording and stock
changes, location management, retained TOTP sign-in, and native/SMART OAuth consent,
token exchange and revocation. Local full CI passed with 58 desktop/mobile cases;
both independent correction reviews accepted that delivery.
[GitHub CI](https://github.com/damacus/med-tracker/actions/runs/37409443565)
succeeded on this exact commit. GitGuardian remains a separate unresolved check;
current incident details are unverified and dashboard sign-in approval is pending.

## Current delivery

Recovery and People workflows are implemented locally. Recovery passes all 14
desktop/mobile cases, including exhausted OTP, passkey-only recovery, one-use
codes, current-account checks and bound OAuth continuation.

People passes eight API cases and its desktop/mobile browser journey. Creation
preserves capacity, carer/grant/Home associations, permissions and audit/sync
effects. Partial updates retain the contract without mandatory `If-Match`.
An actual Rails service probe supplied the exact saved POST/PATCH retry digests,
including filtered email; both replay correctly. New Rust keys still bind the
submitted body. Forced audit failure rolls back every dependent write. A reproduced
lost permission-version increment is corrected with an atomic stored-value
increment. Extra request fields are filtered as Rails does, protected account IDs
cannot be assigned, and an empty filtered person returns 400.

Devin SWE-2 Max approved both Recovery and People corrections. Final combined
`task ci` passed all Rust/static/lifecycle checks, 31 care API cases and all 74
desktop/mobile browser cases. Owned test containers, volumes and networks are
absent after cleanup. The pre-correction partial run is superseded by this result.
The corrected delivery is local until committed and pushed; its hosted checks
have not run yet. Passkey and household administration tests are prepared outside
discovery for the next implementation boundary.

## Whole-migration acceptance

| Area | Implemented evidence | Still required |
| --- | --- | --- |
| Foundation | Original runner, audit, corrective review and exact-head hosted CI gates pass | Publish the updated foundation acceptance record; product and populated rollback gates belong to later areas |
| Persistence | Guarded schema adoption, restricted roles/RLS, persistence and tenant tests, normal seed CLI | Populated Rails rollback and complete schema acceptance |
| Identity | Password/session and bearer tests; OAuth library integration; locally verified TOTP | Recovery, passkeys, remaining security settings and complete client interoperability |
| Care | Dose, stock, medication management; local location API/browser checks | Every remaining household, person, membership, grant, dosage, treatment and sync operation |
| APIs | Actual Loco listener checks for migrated care and OAuth routes | Complete native/FHIR/SMART/MCP contracts |
| Browser | Tera/daisyUI themes and migrated care/OAuth journeys on desktop/mobile | Remaining workflows, accessibility evidence and PWA/offline |
| Workers | Owner-provisioned queue and restricted queue startup/recovery configuration | Every real job, recurrence, reminder, retry/idempotency and crash/drain outcome |
| Release | Retained source/PR inputs and rollback application preserved | Both scratch architectures, complete journeys, populated rollback and obsolete tooling removal |

## Working rules and reports

Use [the current working rules](team-charter.md). Historical relocation/workspace
and source-preservation audits run explicitly through `task migration:audit`.
Do not regenerate historical ledgers for each delivery. Duplicate runners, unused
demos and the proposed temporary worker probe were removed. Keep real database
isolation, process cleanup and side-effect verification.

Migration defects stay in this work. Create follow-up issues only for independent
scope. Useful retained PR inputs are recorded in [pr-inputs.md](pr-inputs.md);
close superseded work only after its replacement is published. Incident #2453
remains separate. Original Rails tests remain the rollback reference; Loco work
does not trigger unrelated RSpec repair.

Update the local reports at `/private/tmp/medtracker-migration-status.html` and
`/private/tmp/medtracker-2450-review-status.md` at each completed slice or material
blocker. Detailed logs stay outside the status page.
Prior delivery history is preserved in Git and the existing evidence artifacts.
Last retrospective: 02:54 UTC on 6 October. Next: 04:54 UTC.
