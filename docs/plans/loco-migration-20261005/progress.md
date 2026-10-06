# Migration progress

Updated 6 October 2026. Complete every slice in [the plan](plan.md).
No complete migration area is accepted. Nothing is merged or deployed.

## Published

The preceding delivery at `45089329dffb3a91cbe64052375c14e2964bd1eb` is pushed to
[draft PR #2451](https://github.com/damacus/med-tracker/pull/2451).
Loco owns root commands, routes, configuration and templates. Rails is independently
runnable under `rails/` as the reference and rollback application.

Published workflows include persistent browser sign-in, permitted household
medication pages, medication create/edit/guarded deletion, dose recording and stock
changes, plus native/SMART OAuth consent, token exchange and revocation.
Local full CI passed with 40 desktop/mobile browser cases.
[GitHub CI](https://github.com/damacus/med-tracker/actions/runs/37403563931)
succeeded on this exact commit. GitGuardian remains a separate unresolved check;
current incident details are unverified and dashboard sign-in approval is pending.

## Current delivery

This delivery adds location management and retained TOTP sign-in.
The actual location create/edit/delete journey passes desktop and mobile.
Eleven focused API cases pass, including saved Rails retry compatibility,
pagination/time filtering, conditional GET, invalid IDs, cached validation replay,
malformed saved-error replay and guarded clinical-history deletion.
Concurrent keyed creation passed on unchanged production: the existing household
lock already prevents duplicate effects. No additional reservation was added.

TOTP passes twelve independent retained-format cases and sixteen desktop/mobile
browser cases, including process restart, concurrent reuse rejection, account
closure, current/old-secret sign-in and missing-challenge redirection.
Devin accepts the bounded TOTP delivery. Pinned Rodauth source and its recorded
independent Ruby oracle disproved the proposed derivation change; the algorithm
and failure policy remain intact.

Final `task ci` passes on the corrected source (14684), including all 58
desktop/mobile browser cases and every Rust suite. Both independent correction
reviews accept their bounded source; their combined-CI condition is satisfied.
Documentation and whitespace checks pass. Owned test containers, volumes and
networks are removed. Hosted verification requires this delivery's exact pushed
commit; earlier hosted success does not certify it.
The next work is recovery-code completion and remaining household/people operations.

## Whole-migration acceptance

| Area | Implemented evidence | Still required |
| --- | --- | --- |
| Foundation | Root Loco commands/routing; independently runnable relocated Rails; published hosted CI | Complete application and rollback acceptance |
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
