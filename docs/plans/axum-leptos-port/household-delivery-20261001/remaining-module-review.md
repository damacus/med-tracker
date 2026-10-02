# Remaining module review

Four existing modules mix responsibilities that should be separated before
final main readiness. The concern is making clinical and authentication changes
easier to review safely; file size alone is not a finding. This is maintainability
work with recommended labels `rust,enhancement`. No new runtime defect or stock
acceptance blocker was established by this bounded source review.

The coordinator accepts these findings into the final planned refactor work,
after stock and assignments are accepted. Complete the three existing assignment,
schedule-write and pause-lifecycle splits before their editors. Do not insert
additional clinical extraction in the middle of the assignment journey. No new
authentication features, policy changes or production activation are authorised.

## Concrete boundaries

| Module | Supported responsibility boundaries | Safeguards |
| --- | --- | --- |
| [dose_occurrences.rs](../../../../rust/api/src/dose_occurrences.rs) | Signed occurrence keys (`key`/`decode_key`, lines 292/307); date/time/pause projection (329–736); representations (737–811); outcome mutations (`mutate`, 1456); replay/sync adapters (`keyed_replay`, 985; `apply_sync_operation`, 2028). | Keep mutation orchestration, reauthorisation and transaction ownership together. Preserve key bytes, dashboard timezone context, recurrence/pause boundaries, ETags, cached errors and sync replay. |
| [dose.rs](../../../../rust/api/src/dose.rs) | History reads/serialization (`serialize`, 442; `index`, 561); effective source and timing rules (`effective_source`, 159; `timing_allowed`, 1093); stock selection/decrement (1235–1464); take creation (`create`, 1574; `create_in_transaction`, 1650); audit/replay (290, 390, 765, 1506). | Preserve `create_in_transaction` for occurrence and sync callers. Keep stock locks, exact option selection, parent aggregation, Decimal arithmetic, UUID replay, timing rejection and take/audit/sync effects within the same transaction. |
| [invitations.rs](../../../../rust/api/src/invitations.rs) | Mail configuration/SMTP delivery (`MailConfig`, 40; `smtp_send`, 51); management actions (`index`, 427; `create`, 460; `destroy`, 631; `resend`, 785); acceptance and membership/grant effects (`record_acceptance_effects`, 1091; `accept_pending`, 1234; `accept`, 1500); representations/replay (165, 317, 1029). | Preserve resend delivery, rollback and commit ordering: SMTP currently executes before transaction completion (962). Do not introduce an outbox or alter delivery promises during extraction. Retain token hashing, tenant context, permission-version effects, accepted retries, audit linkage and no-store responses. |
| [oauth.rs](../../../../rust/api/src/oauth.rs) | Browser session/login/logout (`browser_session`, 501; `login_post`, 765; `logout`, 951); client authorization/consent (`authorize`, 569; `consent`, 1019); code/refresh/revoke (`redeem_code`, 1192; `rotate_refresh`, 1246; `revoke`, 1334); discovery/capabilities/routes (361–467). | Retain shared signing, account checks, lifetime and origin helpers rather than duplicating policy. Preserve signed cookie formats, pending/login intent, PKCE checks, code single use, refresh rotation, locking, revocation and existing exports. Do not add authentication parity features as part of this extraction. |

Line references describe the source inspected on 1 October 2026 and may move
after the planned extraction. The boundaries derive from current functions and
callers; they do not require one child module per row item.

## Earlier review coverage and acceptance

The [eight-file extraction record](../refactor-20261001.md) covers other modules,
including read_resources, people, locations, medication_management, web_pages,
the API facade and web/contract facades. These four were not extracted or accepted
as decomposed by that review. Current [issue #2348](https://github.com/damacus/med-tracker/issues/2348)
and its dosage completion comment list dosage options, assignment writes,
schedule writes, pause lifecycle and portable imports; they do not list these
four. The coordinator will incorporate this accepted additional scope into the
final planned refactor record.

Apply the same acceptance discipline: establish the existing observable baseline,
move cohesive responsibilities mechanically into private modules, preserve
comments and public exports, compare the moved bodies and dependency direction,
run meaningful regressions and the Rust source gate, then obtain independent
review. Keep known baseline failures explicit and do not weaken assertions.
Relevant existing contract suites include doses, schedules, sync, invitations
and oauth; select additional cases only where the moved boundary requires them.
This reviewer performed no runtime, builds, source edits or Git mutations.

Stock requirements and security acceptance remain in
[review-stock-report.md](review-stock-report.md); this report does not change
their verdict or the immutable accepted dosage review.
