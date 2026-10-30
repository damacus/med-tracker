# Person-owned schedules and approved household sharing

Issue: <https://github.com/damacus/med-tracker/issues/2684>

Cross-cutting sharing acceptance is also tracked in
<https://github.com/damacus/med-tracker/issues/2976>; Task4 owns that same journey.

Source base: `234e65ac45d06742354f2ae468354dd6bafbe746`.
Implementation approved by the owner on 28 October 2026. The owner's later
29 October instruction supersedes the earlier Fast approval: every parent,
implementer and reviewer must use verified normal/default service. Current
implementation and routine review use Sol 6.1 High. Astra security review must
also use normal/default service and retain its approved payload scope.
The owner subsequently explicitly approved the iOS authentication migration
needed to align its older client with the current backend. This is Task 3;
the originally numbered sharing Task 3 is now Task 4.

## Spec and owner decisions

ADRs 0013–0017, especially ADR 0014, and the owner's answers in this chat are
the binding specification. Household schedule CRUD and self/declared-guardian
clinical authority already exist; preserve them rather than recreate them.

- A plan belongs to its Person independently of household stock and membership.
- Store the sharer's originating named timezone. Use its seasonal DST rules;
  another household's timezone must not redefine the schedule.
- Match stock through existing internal stable identities or NHS dm+d catalogue
  identifiers, with exact medicine, formulation, strength and stock unit. No
  name/text similarity, unit conversion or invented manual identity fallback.
- One explicit approval per person and household covers current and future plans.
  Invitation acceptance can record that approval with clear consent wording.
- A current explicitly authorised guardian may approve sharing for a dependent
  without joining the receiving household. Record guardian actor and subject.
- Departure ends sharing immediately and permanently for that approval. Rejoining
  requires fresh approval; membership restoration must not revive sharing.
- Current households see current plans, active pause state and latest safety
  status, not another household's detailed history or narrative notes.
- Only self or an explicitly authorised guardian may change a shared clinical
  plan. A manager may explicitly and auditably bind exact compatible local stock,
  without gaining clinical authority.
- Retain original household events and immutable account/person/membership,
  stock/location, plan and dose attribution after departure. No automatic stock
  movement, inferred merge of existing plans, or operational effects from restore.

## Global Constraints

- Follow AGENTS.md and the three repository Rust skills. Fish/RTK, no Python,
  native Task verification and a failing observable test before executable edits.
- Use typed commands/results and distinct IDs, exact Decimal arithmetic, SeaORM
  entities/query builders/relations/transactions for expressible database work.
  Document any exact unsupported RLS/DDL operation narrowly; parameterise values.
- Keep restricted runtime credentials and real PostgreSQL RLS. Recheck current
  account, membership, disclosure and guardian authority at mutation/replay/queue
  boundaries, including after waits. Preserve audit/outbox/stock atomicity.
- Use existing fixture and nextest setup; no new reset framework, coordination
  slots, database adoption, duplicate Cargo caches or global truncation.
- Keep existing local schedule/assignment IDs, portable IDs, HTTP/MCP envelopes,
  ETags, replay behaviour, historical source FKs and supported clients compatible.
  Document deliberate access additions in OpenAPI, clients and ADRs together.
- Preserve current recurrence and DST calculations, signed occurrence validation,
  dose limits, retained pauses/history and no double stock deduction.
- Preserve external worktrees, all unrelated jobs/data and the pre-existing
  `.serena/project.yml` modification. No production or live-data actions.
- Do not implement #2685 history readers, #2845 SMTP, paused #2680 optional
  locations, stock transfer, generic household navigation, or new reports.
- #2686 owns assignment/unassignment completion. Shared helpers must preserve
  assignments and expose reusable interfaces without claiming that issue done.
  Coordinate the necessary schedule sync/restore adapters with #2689 without
  claiming its entire remaining scope complete.
- One implementer at a time; no worker may spawn agents. Read-only task review
  after each implementation, then a broad final branch review. Controller does
  not fix executable code. The owner's later delivery goal authorises controller
  publication after independent review and verification; workers do not publish.
  No GitHub PR/shared-branch merge or deployment. Explicitly recorded local
  feature-branch integrations preserve their history and attribution.
- Before commits verify author, committer and signing identity. If signing is
  unavailable disable it for that commit only. Use Conventional Commits.
  Read exact identity/signing config keys only; never dump Git config or use
  broad --get-regexp output, which can include credential configuration.

## Architecture direction

Retain household execution records and historical FKs. Add a narrow canonical
person-owned schedule/plan identity and household projection links rather than
removing household constraints from inventory/history. Keep prescribed medicine
identity and originating timezone independent of a selected household stock row.
Store attributable sharing approvals with terminal departure cutoffs. Projection
updates target only currently authorised participation, and must not copy private
notes. Reuse existing transaction, audit, sync event and outbox patterns.

An originating household's existing source can be linked without inferring any
cross-household equivalence. Cross-household safety uses canonical plan/occurrence
identity and the smallest status needed for cooldown/count checks, not a global
history endpoint. Edits must invalidate stale occurrence/form assumptions safely.

## Recorded implementation rulings

Current main lacks the canonical Account activation enforcement needed by the
sharing acceptance. The local candidate explicitly includes independently owned
PR2974 at reviewed head `3a46a02bdc38deaf6c37cc5944f9a853b72e42d0` as a dependency.
This does not merge the GitHub PR or establish that main contains it. Keep the
dependency and its future reconciliation visible in evidence and delivery state.

Receiving households use a separate safe plan/safety state, with one row per
canonical schedule UUID locked through SeaORM. They cannot obtain the private
canonical row or clinical editing permission through a stock binding. Clinical
writes require current self/declared-guardian authority; receiving writes require
current approval and database guards limiting their effects to safety state.

The initial privileged safety-aggregate proposal was superseded during Task 4.
Global safety uses minimal immutable administration facts derived from an actual
owned take, its exact source and current approval. Caller-selected foreign history
windows and generic privileged history readers are prohibited. Ordinary reads,
aggregates and locks use SeaORM/SeaQuery. Only unsupported database guard DDL
may use the documented narrow exception, with actor, approval and post-wait
authority checks. Focused passing evidence remains qualified by its source;
the complete integrated candidate still requires final verification.

A guardian-created approval retains its historical actor and declaration, while
current dependent disclosure requires that declaration and source guardian link
remain eligible. Withdrawal or supersession requires a fresh invited approval
epoch. This fail-closed ruling does not rewrite historical local care events.

## Task 1: Exact stock identity for scheduled care

Own `src/models/care/doses/{stock,input}.rs`,
`src/models/care/treatments/source_stock.rs`, one focused shared identity module
if needed, their focused tests and directly affected stock contract wording.

1. Trace the existing internal portable identity and catalogue system/code/
   concept-class fields. Introduce a typed exact identity comparison that admits
   the same concrete stock row and equal complete catalogue identity identifying
   medicine/formulation/strength, with exact stock-unit checks. Administration
   defaults are not proof of medicinal strength. Equal names or missing identifiers must
   never admit a different stock row. Do not parse formulation/strength from text.
2. Use the same comparator for presented stock choices and dose-time validation.
   Preserve current explicit stock choice, tracked dosage-option resolution,
   amounts, sufficient-stock checks and atomic replay/deduction. Task 4 will add
   the separately attributable manager binding for shared plans.
3. Write and observe focused failures first: same-name different identity,
   different system/class/code, different strength/unit, incomplete identity,
   exact identifiers with different display names, same-row compatibility.
   Add a real restricted-runtime acceptance for rejected incompatible selection
   with no take, stock change or successful clinical/stock version; preserve
   rejected-request audit. Retain existing successful exact takes.
4. Self-review, focused native Task checks and git diff --check. Commit this task
   only after checking identity/signing. Report RED/GREEN commands and output,
   changed paths, interface and any concerns. Do not run final full CI yet.

## Task 2: Person-owned schedule continuity in the originating household

Own canonical schedule schema/entities, schedule/treatment command/lifecycle/
projection code, pause/occurrence/dose adapters, necessary personal plan access,
browser treatment surfaces, focused tests and corresponding API/ADR/client changes.

1. Add person-owned canonical schedule identity, prescribed catalogue identity,
   originating named timezone, versioned current clinical state and local
   household execution linkage. Preserve all existing source identities/FKs and
   attributable history. Do not infer shared plans from equal names or medicines.
2. Complete create/read/edit/pause/resume/retire through existing household API/UI
   entry points using that canonical state. Reuse validation and current self or
   eligible guardian checks. Clinical edits must not rewrite recorded dose facts.
3. Keep a person's plan after the originating participation ends. Provide the
   smallest authenticated personal plan read/edit surface needed for continuity,
   without changing personal history readers or granting household credentials
   global access. Existing account-bound browser/mobile identity patterns apply.
4. Add explicit originating timezone capture and pass it to recurrence, occurrence
   and dose safety calculations. Preserve gap/fold behaviour and stale ETag/key
   rejection. A viewer timezone is display-only for the canonical plan.
   Existing sources have no recoverable historical zone: at first authorised
   adoption freeze the configured application zone with explicit
   `adopted_application_zone` provenance. Show the stored zone in read/edit UI
   and permit an authorised correction; do not claim it is the historical origin.
5. Hook departure so later canonical edits do not change departed projections.
   Retain the household's original events and recorded attribution. Do not
   reactivate retired sources or stock on membership restoration.
   Preserve canonical linkage and terminal cutoffs during existing portability
   upserts. Unchanged linked history may restore without operational effects;
   conflicting clinical or current-pause changes must return an actionable atomic
   conflict. Compare a closed execution against its frozen local snapshot, not
   later canonical state. Do not silently reopen pauses or drop conflicts.
6. Failing tests first, then focused App::boot/runtime-role acceptance: ordinary
   create/edit/pause, immutable history, person continuity after last departure,
   denied manager edits, revoked guardian/session, stale forms and timezone/DST.
   Browser proof desktop/mobile under docs/screenshots for the changed journey.
7. Use Task 1 exact stock interface. Keep assignment behaviour unchanged and
   document the canonical interface #2686 can consume. Update directly affected
   OpenAPI, supported client models and ADR wording. Focused checks, self-review,
   identity-verified Conventional Commit and detailed RED/GREEN report.
   Android contract alignment remains in this task. Exact iOS pinning exposed a
   pre-existing authentication migration; the owner approved that additional work
   as the separate Task 3. Its acceptance remains mandatory, not waived here.

## Task 3: Align iOS authentication and its exact current API contract

Owner explicitly approved including this migration after the exact client pin
showed removed production login/refresh/OIDC callers. Own the sibling workspace
`/Users/damacus/repos/damacus/med-tracker-ios`, feature branch
`codex/person-owned-schedules-contract`, base
`488b19306680a3ed17d430dc65a44e1224d52460`, including its preserved uncommitted
exact-pin draft and new wire tests. No unrelated UI/product work or publication.

1. Read iOS AGENTS.md and applicable platform skills, current backend ADR 0015,
   actual native authentication/first-party mobile OAuth routes and existing
   protocol tests. Discover the supported sign-in, token rotation, membership
   selection and logout contract before executable edits. Use current primary
   dependency docs through Context7 as needed; do not recreate removed development
   login APIs or change Better Auth storage/protocol verification for compatibility.
2. Replace the app's removed `createLoginSession`, `refreshSession`,
   `exchangeOidcSession` and related old response-type usage with supported native
   authentication/mobile OAuth. Retain account/person identity, current household
   participation, session revocation, refresh rotation and usable error handling.
   Preserve existing supported user actions through the maintained server flow;
   change native presentation only where the supported protocol requires it.
   Derive its first-party OAuth authority from configured `API_BASE_URL` origin
   per current backend routes, removing obsolete external `OIDC_ISSUER_URL`.
   Keep client ID/callback explicit and validate origin/HTTPS with deliberate
   custom-port/test handling. Document required server-side first-party client,
   scope and redirect registration; no live registration/provider changes or
   hosted sign-in acceptance is included.
3. Preserve required PKCE, state/nonce, one-time code/replay, redirect/client
   restrictions, origin/CSRF and secure credential storage. Prefer existing
   maintained protocol helpers and Apple APIs. No custom crypto, token verification
   or authentication-store proxy. No new third-party dependency without approval.
   Existing dependency updates must be published/pinned and keep lock integrity.
4. Pin the exact tested backend schema/provenance and regenerate with native Task
   entries, never a hand-edited generated client or falsely labelled delta overlay.
   Replace obsolete validator/test expectations only with the actual supported
   contract; preserve negative PKCE, malformed-response, empty/ambiguous portable
   payload and cross-account rejection assertions. Resolve generator interoperability
   through maintained tooling or semantics-preserving schema definitions, not by
   disabling validation or weakening the wire contract.
   The evidenced Kotlin okhttp4/Moshi generator limitation permits one explicit
   Android generator-input adapter for `PortableImportUnprocessableResponse`:
   retain the exact full canonical pin, derive only this model's previous flat
   optional typed fields for generation, and document canonical/derived hashes
   and transformation. This preserves the existing Android decoder limitation;
   do not claim equivalent validation, alter other schemas, hand-edit output,
   add a shadow type or change HTTP stacks/dependencies. Canonical/backend and
   Swift null/empty/ambiguous/extra-field rejection remain strict. Use existing
   native tooling with reproducibility, unchanged-other-schema and wire tests.
   The necessary PersonCreate adapter may offer only supported `minor` and
   `dependent_adult` creation and must collect an explicit, initially unchecked
   guardian declaration using current backend/browser wording. Never infer or
   silently submit attestation. Remove/disable unsupported `adult` creation;
   adults retain the existing account/invitation journey. No unrelated People
   feature or new navigation is included; verify this changed form with native UI.
   New drafts start with no birth date. Both supported dependent types require
   an explicitly selected/confirmed date before Save or a create request, matching
   current backend NaiveDate/age validation. Opening/cancelling the form/picker
   must not commit a made-up date. Do not change the backend/schema requirement.
   Guardian confirmation remains a separate initially unchecked choice.
5. Observe failing regression tests before changing production behaviour. Prove
   request/response wire compatibility, authentication success/failure, stale or
   revoked credentials, refresh/logout, wrong state/verifier/code replay and
   household scoping using existing Swift/native backend acceptance facilities.
   Preserve the new schedule roundtrip cases and required Person constructor
   semantics. Do not resurrect obsolete backend auth tests or broad fixture resets.
6. Use native `task api:contract`, `api:generate`, `api:check`, `api:test` and the
   necessary app build/test gates. Keep Swift 6 strict concurrency/warnings-as-errors.
   The owner now requires 27.1 everywhere: Xcode27.1, iOS27.1 deployment/test
   destinations, CI and guide pins. This supersedes the old26.6/iOS26 guide.
   Preserve existing runtimes/devices when adding needed27.1 runtime support.
   If visible auth UI changes, verify on iPhone in light/dark and accessibility
   text size with actual simulator screenshots. The owner explicitly deferred
   iPad visual verification because no supported27.1 iPad runtime is available;
   record it as deferred/unverified, retain universal iPad compilation, and do
   not waive refresh fixes, other UI checks, full CI or security review.
   Clearly distinguish automated/local/simulator proof from physical authenticator,
   live provider and device acceptance, which remain excluded.
7. If a semantics-preserving backend schema change is necessary for the pinned
   generator, own only the corresponding contract/tests and Android exact repin;
   do not introduce unrelated backend authentication behaviour. Coordinate its
   actual commit provenance before final client generation.
8. Read exact Git identity/signing keys only. Commit with verified Dan Webb identity
   and signing as required by the sibling guide; no pushes despite its default
   publish instruction. Report both repository heads if changed, full RED/GREEN
   evidence, native UI proof, unresolved physical/live limits and a sanitised report.
   A fresh independent review is required before starting sharing Task 4.

### Native migration reference and acceptance coverage

| Area | Established reference / required preservation |
| --- | --- |
| Journeys and roles | iOS488b193 password/OIDC sign-in, refresh, household choice and logout; current backend native/mobile OAuth is authority, not removed dev routes |
| Business rules | Current membership/account revocation and household request binding; no new clinical permission from authentication |
| Data | Existing opaque-token/Keychain/session handling; preserve clinical data, no backend adoption/reset or legacy credential import |
| UI | Existing Authentication views/AppRootView are baseline; retain features/layouts where supported and capture actual simulator comparisons for necessary changes |
| Contracts | Exact current OpenAPI pin plus actual native OAuth protocol; strict schedule metadata roundtrip and existing negative portability/security assertions |
| Background work | Preserve refresh/retry coordination; no new jobs, notifications or provider integration products |
| Runtime/cutover | Swift6; owner-approved Xcode27.1, iOS27.1 deployment/tests, CI/guide standardisation; no release/deployment |
| Acceptance | Native Task package/app gates, restricted-runtime backend protocol evidence, iPhone visible checks; owner-deferred iPad visuals/universal compile retained; physical/live provider proof excluded |

## Task 4: Approved second-household sharing and safe local recording

Own sharing approval schema/entities/domain operations, invitation consent and
dependent sharing UI/API, guardian approval checks, household projection fan-out,
explicit shared-plan stock binding, cross-household occurrence/safety integration,
affected schedule sync/queued/restore adapters, focused tests/contracts/docs.

1. One attributable person-household approval covers existing/future plans.
   Record clear consent atomically with invitation acceptance for self. A current
   explicit guardian may approve the dependent without joining the destination.
   Use a dedicated dependent-sharing invitation/acceptance path built on the
   existing manager-issued, recipient-bound invitation infrastructure. Do not
   relax existing self-invitation rejection of injected subject/dependent IDs.
   The destination manager must invite and the eligible guardian must consent;
   no arbitrary household-ID enrolment, duplicate Person or guardian membership.
   Never use household management, legacy grants or relationship labels as
   substitute guardian authority. Recheck declaration validity after waits.
2. End the approval on departure. Rejoining/restoring membership requires a new
   consent record, never reopening the old one. Suspension must deny all current
   access; retain a distinct terminal departure cutoff. Revoke stale queued writes
   and cached replay authority. All effects and audit must roll back together.
3. Expose approved current plans, pause status and minimal latest administration
   safety status. Exclude foreign detailed history, private notes, pause narratives
   and future updates to departed households. Preserve originating timezone.
4. A current receiving-household manager explicitly binds local stock to the
   exact prescribed catalogue identity using Task 1 semantics. Audit the decision
   and revalidate identity/unit on recording and after medication changes. A stock
   binding never permits clinical edits. No automatic communal stock movement.
5. Connect canonical occurrence identity and dose limits across active household
   projections: competing recordings cannot double-administer, bypass cooldown
   or double-deduct. Deduct only selected local stock and retain actual household,
   membership, source and stock/location attribution. Preserve signed keys/replay.
6. Adapt necessary existing schedule change feeds, queued commands and portability
   to these boundaries; historical restore remains side-effect-free. Coordinate
   interfaces for #2689; do not add unrelated sync products or history readers.
7. Failing focused acceptance first: consent required and once-only, authorised
   guardian outside B, management denial, two-household recording race, exact stock
   mismatch, audit rollback, approval withdrawal/departure/rejoin, stale replay,
   no foreign narrative and originating DST. Verify browser consent/share/stock
   journeys at desktop/mobile sizes with accessible labels/errors/focus.
8. Update affected OpenAPI/client/MCP contracts and accepted design wording.
   Self-review, focused checks and identity-verified Conventional Commit. Run final
   rtk task ci and docs build once the complete reviewed candidate is frozen;
   correct concrete failures through the controller's review/fix loop.

## Completion gates

Task-scoped independent reviews must give spec and quality verdicts against the
whole task commit range. Final review sees all changes and ledger rulings. Use
normal-tier Astra for the cross-household permission/security review if a supported
normal-tier route is available; never dispatch Astra through priority-only tools.
Full application `rtk task ci`, documentation `rtk task docs:build`, diff checks and
actual changed UI screenshots are required before claiming completion. No physical
device, deployed/provider or production evidence is asserted. Retain local work
and report publication separately. The owner's later delivery goal authorises the
controller to publish verified independent slices and address actionable PR
comments; no merge/deployment is authorised.

### Independent exact-stock candidate

The exact-stock slice is prepared separately on `codex/fix-exact-dose-stock`
against main `a621dbc17576a98d5b54717397dda7dcdd9a23e7`, at candidate
`8cfd6c6e895043023526c2640050893a16347172`. Original reviewed code was copied
unchanged; the only follow-up moves an unchanged test module after production
items for strict Clippy. That mechanical commit uses the authorised unsigned
fallback after 1Password failed; global signing remains enabled.

Full `task ci` passed on this candidate: 186 unit, 1031 integration and 62 browser
tests, plus the existing JavaScript and harness checks. Integration skipped69,
browser skipped3; no skipped case is claimed as executed. `task docs:build` and
diff checks passed. CI-generated screenshots were preserved separately and
restored. All hosted checks subsequently passed on that exact candidate, including
both architecture image builds. No skipped case is claimed as executed.

The owner explicitly approved the exact-stock private payload for default-tier
Astra review after the initial automatic rejection. Its substantive read-only
review found no introduced blocker. Full-range Sol spec and quality reviews also
approved. The inherited unsupported taper-unit finding was reproduced and is
being addressed in the ongoing shared-dose work; it was not introduced by this
slice. No rejected-route workaround was used.

[PR 2983](https://github.com/damacus/med-tracker/pull/2983) was published against
main and then squash-merged by the delivery controller at
`6b7c051b1ec645049f1885afba564f0133c42f34` on 28 October 2026. This records local,
hosted and merged evidence separately. No deployment is claimed. Sharing and
final backend/native generation, verification and publication remain incomplete.
