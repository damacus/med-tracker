# MedTracker Rust refactor audit

Read-only audit of the changed Rust code in PR #2346. Writer candidates are in
progress; this is an allocation recommendation, not final candidate acceptance.
The reviewer ran no builds or tests and made no product or Git changes.

## Justified remaining work

### Web crate root: separate auth and medication rendering

`rust/web/src/lib.rs` combines several distinct responsibilities: authentication
components and OAuth consent, shared HTML document construction, medication
inventory/detail and two administration dialogs, a legacy journey dashboard,
asset accessors and a standalone preview router. The medication block begins at
`MedicationCard` around line 178 and extends through the journey dashboard around
line 500; auth/consent definitions are separated from their public render entry
points by that large block. Navigating or changing either family requires passing
unrelated code, unlike the existing dedicated people/locations/dashboard modules.

Recommended extraction: medication models/renderers into `medication.rs`, auth
components/consent into `auth.rs`, and shared document construction into a private
`document.rs` if both render families require it. Keep root explicit re-exports
for the existing public model/render APIs; retain preview route and asset entry
points in the root or one small preview module. Keep `render_journey_dashboard`
with the existing medication journey views. Do not consolidate it with the full
dashboard renderer: they have different input models and observable markup.

Both inline unit tests must remain discovered and continue checking native login
and rejected-dose snapshot/dialog restoration. Existing external API and web
tests import models and render functions from the crate root, so root re-exports
avoid caller churn and unnecessary public-module exposure.

### Contract harness root: separate fixture schema from transport

`rust/contract-tests/src/lib.rs` mixes a large flat fixture JSON DTO and fixture
loading with the cookie/bearer HTTP Target and URL/write safety validation. The
Fixture is around lines 11–378, Target begins at 380, checked URL validation is
at 878, and fixture loading is at 889. These are independent navigation domains,
not one long algorithm.

Recommended first extraction: `fixture.rs` holds the unchanged Fixture and
fixture loader; `target.rs` holds Target and its origin/write safeguards. Keep
root explicit `pub use` exports for `Fixture`, `fixture` and `Target`. Preserve
the flat fixture names, required fields, Deserialize behaviour and missing-file/
invalid-JSON failures; grouping the external schema is a separate behaviour
change and is not needed for this refactor.

If transport navigation still needs subdivision, use Target child modules for
browser/form requests and JSON requests, leaving origin validation, URL joining,
authorization and local-write protection together. Descendant modules can retain
access to private Target fields/helpers without exposing them publicly. Preserve
all per-method distinctions: optional bearer versus cookie context, HTML versus
JSON Accept, actual Origin, CSRF, If-Match, idempotency key, X-Forwarded-For and
loopback guards. Several similar-looking methods deliberately model different
contract/security failures; avoid replacing them with a generic request builder
or removing an alias during a mechanical split. Keep four URL safety tests in
the Target module and ensure they remain under a registered cfg(test) module.

## Other changed Rust modules

No additional mandatory decomposition is justified solely by the remaining file
sizes. The new web people, locations and medication-management render modules
each own one resource's draft and list/detail/form rendering. The household shell
and i18n adapter each have a clear boundary; the i18n adapter combines locale
negotiation, restricted catalogue lookup and interpolation as one translation
service. The capability route is small and cohesive. The pure OTP compatibility
helper and its vector suite are already separated.

The existing browser medication adapter has separable pure draft/payload helpers
and request orchestration, but it is already resource-specific and the current
web-pages writer owns the relevant boundary. Any extraction there should be
driven by that writer's final dependency graph rather than another simultaneous
owner. The household contract integration suites group observable scenarios by
resource/journey; splitting test files merely to reduce line counts would not
improve their responsibility boundary.

## Review hazards for all writers

- Preserve public exports and meaningful visibility. Moving a function must not
  make sensitive internal helpers public merely to satisfy imports. Check changed
  `super`/`crate` references and route registration after moving nested modules.
- Register every new module. Integration tests in `tests/` are discovered as
  top-level targets; files moved into subdirectories need explicit modules. Inline
  unit test modules moved out of the root still need cfg(test) registration.
- Relative `include_str!`/`include_bytes!` paths depend on the new source location;
  assets and embedded locale catalogue paths must still resolve to the same files.
- Preserve transaction lifetime, lock sequence, membership-version invalidation,
  ETag precision and actual If-Match/Origin/CSRF forwarding. Extracting an operation
  must not move a lookup or write outside its transaction/authorization scope.
- Preserve existing comments verbatim while moving code. User instructions forbid
  adding or removing comments; a mechanical extraction is not permission to tidy
  them or create new explanatory comments.
- Avoid duplicate domain types, broad wildcard exports and helpers with ambient
  request state. Pass existing state/context explicitly across real boundaries.
- Keep the established integration baseline and assertion strength. Runner owns
  shared builds/tests after all writers freeze. Source acceptance cannot replace
  final HTTP/browser verification, including strict mobile overflow and exact dose,
  stock, replay, CSRF and policy cases.

## Pending final review

After writer freeze, compare the whole diff independently for changed semantics,
public API and tests, then review runner results. No final refactor acceptance is
claimed by this audit.

## Frozen candidate source review

Reviewed all eight completed decompositions against the current committed HEAD:
API root, People, Locations, Medication management, read resources, browser pages,
web root and contract harness. No blocking source finding was identified.

A read-only lexical comparison accounted for all function/struct/enum symbol
names and bodies in each family: 41 API-root names, 25 People, 33 Locations,
36 Medication management, 52 read-resource, 78 browser-page, 32 web-root and
49 harness names. None are missing or newly invented. Function bodies compare
unchanged after whitespace normalisation except route-handler module qualification
and the required relative asset path depth. Struct body changes are only scoped
field visibility needed between private modules. These checks support mechanical
equivalence; compilation and runtime remain the runner's responsibility.

Raw comment multisets are identical across all eight families. Test attributes
remain 7 read-resource, 4 browser-page, 3 API-root, 2 web-root and 4 harness;
mutation families had no inline test attributes before or after. All moved test
function bodies are preserved. Explicit module declarations register the moved
unit tests; top-level integration test targets stay in place. All seventeen moved
browser asset include paths resolve to the identical existing canonical files.

The facades preserve the former callable names with explicit exports/imports.
Public web models/render functions remain at the crate root, and the fixture DTO/
loader and Target retain their root imports. API AppState/connect/router and the
public compatibility module remain. Internal module visibility stays limited to
the appropriate parent subtree or crate; no browser API state/header fields or
Target transport fields become externally accessible. Existing CRUD browser
children retain their private parent interfaces. Per-method harness transport
headers, cookie/bearer choice, local-write checks and URL safety tests are unchanged.
Mutation body equivalence preserves transaction boundaries, locks and version
updates. The runner may make formatting/import corrections without changing these
accepted behaviours; any semantic correction needs a subsequent review.

The harness Target remains about 545 lines but owns one transport boundary; no
further size-only split is warranted. The web root retains small preview/journey
entry points while actual auth/document/medication concerns are separated. Its two
dashboard models remain distinct.

## Remaining function-level cohesion recommendation

`web_pages/dashboard.rs::dashboard` still owns API reads and response/error policy
alongside about 180 lines of pure task/outcome/history/stock projection. This is a
real remaining boundary, rather than a file-size complaint. A per-source task
projector and pure history/stock builders could live in dashboard_projection.rs,
leaving authenticated reads, selection validation, cookie propagation and final
response in the handler. Preserve occurrence fetch ordering, timezone task-local
context, permission/stock rules, PRN state ordering, metric inclusion and history
local-day filtering.

That function is unchanged existing behaviour, so it is not a correctness blocker
for these mechanical module moves. The coordinator should record it explicitly as
a follow-up or authorise one further bounded extraction; do not invent broad data
abstractions or loosen the established integration assertions to make it smaller.

## Verification status

Source acceptance is complete for the frozen eight-module extraction candidate.
Serial formatting/compilation/unit and HTTP/browser verification are pending.
No reviewer build/test command, product mutation or Git mutation was performed.
External GitGuardian false-positive disposition remains a separate tracked
publication status and is not changed by this source review.

The coordinator's two new contract-harness formatting tasks were also reviewed.
They check or write formatting from `rust/contract-tests/src/lib.rs` with explicit
edition 2021, matching the contract crate. Module-tree formatting includes the new
fixture/Target children without selecting unrelated integration test files. No
blocking source/configuration finding was identified.

The coordinator accepted the dashboard function-level recommendation and its
owner is performing the bounded pure-projection extraction. Final source clearance
now awaits that candidate; the remaining eight-family review is complete.

## Final projection review and runner release

The final dashboard extraction is source-accepted. `project_source_tasks` uses a
borrowed, resource-specific input record and updates the existing person/metric
collections. Medication-name fallback, empty paused routine handling, PRN
calculation, occurrence state precedence, taken/not-taken placement and metric
inclusion retain their former branch bodies. The same captured now and timezone
are passed; deriving today from those values introduces no new clock read.

`dashboard_history` retains selected-person and local-day filtering plus exact
medication/person/dose/time projection. `dashboard_stock` retains source stock-ID
sorting/deduplication and the same inventory fields. The handler still owns API
read order, source-ID validation, occurrence reads inside the existing task-local
timezone, API/malformed-response error policy, person-row sorting, cookies and the
final rendered response. No projection helper performs a new query or request.
New visibility is limited to the browser-page parent subtree.

No blocking finding remains in the final decompositions, projection boundaries or
format tasks. The independent reviewer released the serial runner for formatting,
compilation/unit checks and full HTTP/browser regression verification. Runtime
acceptance and publication still depend on those final results; source clearance
does not claim them complete. The existing externally failed GitGuardian
synthetic-vector incidents remain tracked separately.

## Broader 72-case HTTP run classification

The candidate broad `api:acceptance` run reports 56 passes and 16 failures in
`/tmp/medtracker-refactor-http-base-raw.log`. This run is RED; it does not replace
the previously accepted bounded journey run. No production/test edit or assertion
weakening is authorised by this classification.

| Count | Category | Exact evidence and source classification |
| --- | --- | --- |
| 4 | Dosage setup fails before intended checks | `dose_mode_transition_api.rs:41` and `management_sync_events_api.rs:128` directly INSERT dosage rows without `default_dose_cycle`; current schema requires a non-null value. The helpers and schema are unchanged from committed pre-refactor `dfab7fe8`. |
| 7 | Timestamp contract mismatch | `medication_stock.rs:57` asserts timestamp string length 20. Actual length is 27 from microsecond medication `updated_at`, deliberately introduced before refactoring for ETag precision. This is not a table/record-count failure. The committed formatter already uses RFC3339 microseconds; the helper is unchanged. |
| 2 | Source projection expectations disagree | `source_capabilities_api.rs:376` expects stock ordering to reverse after a location change, whereas the committed projection sorts IDs ascending. Line 293 expects paused-source eligible stock IDs empty, while the committed stock loader filters by record permission/signature/supply and does not exclude pause here. Those bodies and tests are unchanged; whether contract or implementation should change is a separate decision. |
| 1 | People pagination contract mismatch | `web_reads_api.rs:332–339` requests People `per_page=999` and expects clamp-to-100 with 200. Both committed/candidate People handlers call the shared `parse_location_page` validator, which rejects out-of-range pagination with 422. This is a People request, despite the helper name. |
| 2 | Viewer dashboard login setup fails | `web_session_api.rs:209` requires the login redirect destination to return 200; both failures use the viewer fixture and receive dashboard 503 before the permission scenario runs. Login POST succeeded. Dashboard dependency/setup limitations were already documented under #2345; the exact failed dependency is not identified by this log. |

All six failing integration test source files, provisioner, relevant schema and
unchanged external endpoint modules have no diff against `dfab7fe8`. Independent
inspection confirms the relevant moved timestamp, pagination and stock projection
bodies retain committed semantics. Source evidence strongly indicates these 16
failures predate the mechanical refactor. That remains provisional until the
coordinator's isolated baseline clone completes the identical 72-case command.
Only matching baseline runtime can establish the exact pre-existing failure set;
do not label broader acceptance GREEN or fix unrelated behaviour to conceal RED.

## Conclusive pre-refactor differential result

The coordinator's clean baseline at committed
`dfab7fe8463a032102da203596c4720723099ef5` completed the identical
`api:acceptance` selection. This reviewer independently compared
`/tmp/medtracker-refactor-baseline-http-raw.log` against the candidate raw log.
Both report 56 passes and 16 failures. All sixteen failing test names and all
sixteen assertion source locations match exactly. There is no candidate-only
failure in this differential run. The coordinator records baseline snapshot
`453d76d198e59dd96a22373c8953a0555feeda615951f87e33a11e2f0d5e96dc` and
candidate snapshot beginning `dc8f260`.

The earlier provisional classification is now confirmed as pre-existing runtime
RED rather than refactor-introduced behaviour: dosage setup, timestamp precision
expectations, source-capability ordering/pause expectations, People pagination
and viewer dashboard setup have the same failures on committed source. This
does not classify every mismatch as harmless or accept broader API parity. The
72-case parity audit remains RED and requires a separate follow-up issue; the
coordinator will link the known viewer dashboard limitation to #2345.

The coordinator records the required final `task ci:rust-port` gate GREEN after
the dashboard projection extraction. That is distinct from the broader optional
parity audit above. Source review and matching baseline runtime support unchanged
refactor behaviour. Narrow resource/household HTTP contracts and household/dose
browser regressions are still running and must pass before scoped publication.
No broad test/policy edit, fixture workaround or assertion relaxation was made
by this reviewer. External GitGuardian status remains separately tracked.

## Final bounded refactor acceptance

The final decompositions and dashboard pure projections are accepted with no
blocking source finding. Existing caller exports, scoped visibility, response
headers, write/transaction boundaries, moved comments/test bodies and asset paths
were preserved. The final `contract-harness-test` task runs the locked contract
crate's `--lib` tests explicitly; it does not merely compile integration tests.
Independent inspection of `/tmp/medtracker-refactor-harness-test.log` confirms
all four approved-origin/URL rejection unit tests executed and passed.

The coordinator's final immutable snapshot is
`082028057307078aec8296ee10983459c89456a0795493cf247ae06ca1fa79ba`.
It differs from the earlier candidate snapshot only by that Taskfile test target;
production Rust source is unchanged. The required Rust gate is recorded GREEN.
People's 17 and Locations' 29 focused HTTP cases are GREEN. Final household
acceptance is 15/15 HTTP and 21/21 browser GREEN. This reviewer inspected the
browser raw log `/tmp/medtracker-refactor-household-browser-raw.log`: zero failed,
skipped or cancelled tests, with desktop/mobile household writes, native draft
retention, dose/history/stock, CSRF/replay, taper, Escape focus and foreign-resource
guards passing. The strict mobile width assertion was retained. Runtime evidence
therefore supports unchanged behaviour for the bounded refactor.

Two additional existing parity audits remain RED, separately from that result:

| Selection | Candidate and committed baseline | Evidence and disposition |
| --- | --- | --- |
| Focused Medication contract | 7 passed, 2 failed on each | Both fail at `openapi_medications.rs:58` exact key-set comparison. `barcode`, `friendly_name` and `warnings` are returned by the committed `dfab7fe8` serializer as well as the candidate, and are optional nullable properties in the unchanged authoritative `Medication` OpenAPI schema. The static test key list omits those intentionally added fields. No serializer/policy regression or spec change was introduced by the refactor. Raw logs: `/tmp/medtracker-refactor-http-medications-raw.log` and `/tmp/medtracker-refactor-baseline-medications-raw.log`. |
| Read-completion contract | 21 passed, 2 failed on each | Both fail at `openapi_read_completion.rs:110` and `:218`, actual `private, no-store` versus exact `no-store` expectation. The same failing names, locations and header values occur on the clean committed baseline. Raw logs: `/tmp/medtracker-refactor-http-read-completion-raw.log` and `/tmp/medtracker-refactor-baseline-read-completion-raw.log`. |

The 72-case audit remains 56/16 on both versions, as documented above. These
existing fixture/expectation/policy-parity gaps require follow-up under #2347,
with known viewer dashboard limitations linked to #2345. This acceptance does
not claim broader API parity or the broader product journey complete. No audit
assertion, fixture, policy or production behaviour was changed to make those
audits pass.

No finding blocks publication of the scoped maintainability refactor on source
or relevant local runtime grounds. The externally failed GitGuardian synthetic
OTP-vector findings still require legitimate integration-owner disposition;
they remain a separate unmerged-PR status and were not suppressed or rewritten.
