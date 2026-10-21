# ADR 0018: Defer SMART integration and record future healthcare interoperability uses

- Status
  Accepted (deferral and planning boundary; use cases are exploratory)
- Date
  2026-10-21
- Amends
  [ADR 0010](0010-external-integration-architecture.md) for fresh Rust delivery

## Context

Issue [#2670](https://github.com/damacus/med-tracker/issues/2670) combines
first-party OAuth acceptance with an unspecified SMART contract. Earlier plans
deferred SMART from the first production gate but retained it in the full
migration goal. That leaves no concrete external consumer or acceptance journey
for deciding when SMART is complete.

The owner wants to explore both sharing MedTracker records and bringing external
records into MedTracker. Those are separate journeys with different providers,
permissions and data handling. Historical Rails endpoints and protocol tests
show prior intent; they do not prove accepted Rust behaviour or establish which
future integrations are useful.

## Decision

Defer SMART implementation until after the MedTracker migration. SMART delivery
does not block migration completion. This supersedes the earlier requirement to
finish SMART within the full migration goal, not just its first production gate.

First-party OAuth sign-in, consent, per-session household isolation and revocation
remain migration responsibilities. Shared OAuth code must retain existing
credential restrictions. This decision neither removes existing functionality
nor expands access, advertised capabilities or supported contracts. It authorises
no application, schema, deployment or live-data changes.

Use the catalogue below to choose future work. Its entries are possibilities,
not approved implementation requirements or a commitment to support every
healthcare system. No particular external system, automatic treatment update,
FHIR write capability or background service is approved here.

Recommend separating the deferred SMART follow-up from first-party OAuth
acceptance in #2670. This record does not change or close that GitHub issue and
does not claim the remaining OAuth acceptance checks have passed.

## What the standards contribute

[FHIR](https://hl7.org/fhir/R4/overview.html) defines structures and interfaces
for exchanging healthcare information. The historical MedTracker adapter targets
R4; a future integration must establish the partner's actual version, profiles,
terminology and supported operations.

[SMART App Launch](https://hl7.org/fhir/smart-app-launch/) builds on OAuth to
authorise apps to use FHIR data and convey launch context, such as a selected
patient. It supports apps launched independently or from an electronic health
record system. These standard capabilities are not claims about MedTracker's
Rust implementation.

SMART does not itself supply access to an NHS, GP or hospital system, match
external patients to MedTracker people, reconcile conflicting records, or grant
authority to change treatment. Those require provider arrangements and explicit
application decisions. A downloaded report may need neither SMART nor FHIR.
FHIR remains an external adapter, separate from first-party product sync and
Rust-native portability under [ADR 0016](0016-fresh-rust-without-legacy-compatibility.md).

## Candidate uses after migration

### External apps read MedTracker records

Benefit and direction: a person uses another app to inspect their medication or
administration history; records flow out of MedTracker. SMART could authorise
the app to read a bounded set of FHIR resources.

Dependencies and decisions: identify a registered consumer and its resource
requirements. Decide which person and history may be disclosed, how consent is
presented, how long access lasts, and how revocation affects future reads and
already copied data. Household membership alone must not disclose another
household's detailed history.

### Share a report with a clinician

Benefit and direction: a clinician reviews a medication list or administration
summary supplied by the person; information flows out. A readable download is
a distinct option from ongoing connected access. SMART could serve the latter;
it is not required merely to generate a report.

Dependencies and decisions: establish the recipient's accepted format, delivery
channel, required date range and content. Resolve who can export whose records,
whether notes are included, and what audit evidence is retained. Do not imply
that revoking a connection can retract an already downloaded report. Reuse the
relevant constraints in [ADR 0017](0017-personal-history-download-authorisation-and-audit.md).

### Send records into a clinical system

Benefit and direction: a GP or hospital receives medication and administration
information without manual re-entry; records flow out. FHIR might represent the
data, and SMART might authorise supported writes, but the receiver must offer
an appropriate ingestion interface.

Dependencies and decisions: establish provider onboarding, accepted profiles,
patient identifiers and receiving permissions. Decide whether submissions are
patient-reported evidence or changes to the clinical record, who approves them,
and how acknowledgements, corrections, duplicate delivery and rejection work.

### Retrieve prescriptions and medication lists

Benefit and direction: a person avoids retyping information from a healthcare
provider; records flow into MedTracker. MedTracker could act as a SMART client
of that provider's FHIR service.

Dependencies and decisions: identify an accessible provider, client registration,
test environment and supported medication resources. Establish patient matching,
proxy or guardian access, source attribution, freshness and what happens when
the external permission expires. Receiving a prescription does not itself
authorise creating or altering a MedTracker treatment schedule.

### Review and reconcile incoming information

Benefit and direction: incoming records become useful treatment information
without silently overwriting local decisions. FHIR can carry source data; SMART
can permit retrieval. Reconciliation is MedTracker application behaviour.

Dependencies and decisions: define an authorised reviewer, exact medicine and
unit mapping, duplicate detection, conflicts, corrections and approval evidence.
Only the person or an explicitly authorised guardian may change a shared
treatment plan under [ADR 0014](0014-household-inventory-and-personal-care.md).
Automatic treatment changes are not approved. Imported historical events must
not replay stock deductions or live reminders.

### View external records alongside MedTracker care

Benefit and direction: a person consults external information without changing
their MedTracker treatment plan; records flow in for reference. SMART/FHIR could
support retrieval without local clinical writes.

Dependencies and decisions: choose the source and displayed record types. Decide
whether data is fetched on demand or stored, how source and age are shown, who
else may see it, and what remains available after disconnection. Displaying a
record must not imply that it has been reconciled or adopted as current treatment.

### Launch MedTracker from a clinical system

Benefit and direction: a clinician opens MedTracker from an existing clinical
workflow with patient context supplied by that system. SMART EHR launch could
convey that context; subsequent data exchange needs its own agreed scope.

Dependencies and decisions: identify the host system and supported launch mode.
Resolve clinician identity, mapping to a MedTracker person and authority to see
their records. A supplied patient identifier or successful external login must
not automatically create household membership or care-sharing permission.

### Automated exchange and future write-back

Benefit and direction: repeated transfers reduce manual work; data could flow
either way. [SMART Backend Services](https://hl7.org/fhir/smart-app-launch/backend-services.html)
supports pre-authorised service access without interactive approval at each run.
It is a separate option from continuing a user's delegated connection and does
not imply that the target supports writes.

Dependencies and decisions: establish the partner's service authentication and
permissions, responsible operator and permitted population. Define scheduling,
live revocation checks, retries, replay protection, reconciliation, audit and
failure handling. Write-back needs explicit source authority and conflict rules
to prevent two systems repeatedly overwriting each other. Bulk export, population
analytics and automatic treatment updates require separate scope approval.

## Conditions for revisiting implementation

After migration, select one complete journey rather than a generic requirement
to implement SMART. Before approving that slice:

1. Name the user, external system, data direction and useful outcome. Confirm
   supported interfaces, onboarding requirements and access to synthetic test data.
2. Define patient matching and mismatches, account and guardian authority,
   disclosure across households, and the permissions needed for each operation.
   Management of a household is not guardianship; apply ADRs
   [0013](0013-accounts-people-and-households.md),
   [0014](0014-household-inventory-and-personal-care.md) and
   [0015](0015-authentication-membership-and-tenancy.md).
3. Decide provenance, freshness, retention after disconnect, reconciliation,
   corrections and which source may change treatment. Mark genuinely irrelevant
   questions explicitly for the chosen journey.
4. Specify consent, revocation, audit, denied access, partial failure and replay
   behaviour. Preserve current authorisation checks and RLS; external scopes
   cannot override MedTracker authority.
5. Agree acceptance evidence with the real partner interface or its supported
   sandbox, including patient mismatch, revoked access and successful data use.
   Select protocol versions and maintained libraries from that evidence.

## Alternatives and consequences

Completing the historical standalone server contract during migration would
commit to an outward-facing integration before choosing a consumer. Treating
SMART as the whole interoperability feature would leave inbound workflows,
reconciliation and provider access unspecified. Both approaches are rejected.

The migration can finish independently of SMART. Future integrations remain
discoverable without becoming migration acceptance gates. They will require
fresh product decisions and verification; preserved source, screenshots and
earlier tests are reference evidence rather than proof of interoperability.
