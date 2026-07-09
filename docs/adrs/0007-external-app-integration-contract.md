# ADR 0007: External App Integration Contract

- Status: Accepted
- Date: 2026-07-09

## Context

MedTracker now exposes two API surfaces that solve different problems:

- `/api/v1` is the MedTracker product API. It uses portable MedTracker IDs,
  household-scoped routes, API sessions, API app tokens, sync primitives,
  backup/export endpoints, household administration, and hosted MCP support.
- `/api/fhir/R4` is the interoperability API. It exposes FHIR R4 read/search
  resources for external healthcare systems that expect FHIR resource shapes,
  `application/fhir+json`, `OperationOutcome`, and searchset Bundles.

Issue #551 originally grouped OAuth API access, pragmatic JSON endpoints, and
FHIR R4 reads into one handoff plan. Subsequent API work implemented the hosted
MedTracker API, MCP endpoint, and FHIR R4 read/search baseline, but it did not
implement a third-party OAuth authorization server or SMART on FHIR app launch
contract.

Without a written split, first-party clients, automation tools, and external
healthcare integrations could all be directed at the same endpoint family even
though they have different identity, consent, payload, and compatibility needs.

## Decision

MedTracker will publish separate contracts by audience:

| Audience | Contract | Primary path |
| --- | --- | --- |
| First-party mobile clients | MedTracker hosted API with OIDC-to-API-session exchange, portable IDs, sync, backup/export, and household routes. | `/api/v1` |
| First-party CLI and local automation | MedTracker hosted API using API sessions or API app tokens, gated by `GET /api/v1/capabilities`. | `/api/v1` |
| Hosted MCP clients | Streamable HTTP MCP endpoint using the same bearer credentials as `/api/v1`, read-only by design. | `/mcp` |
| External healthcare apps and provider systems | FHIR R4 read/search resources with standard FHIR HTTP semantics. | `/api/fhir/R4` |
| Internal/admin tooling | Web UI and explicitly documented MedTracker admin API operations. | Web routes and `/api/v1` admin routes |

External healthcare integrations should use FHIR R4 first. They should not
depend on MedTracker `/api/v1` payload shapes unless they are also acting as a
first-party or explicitly trusted MedTracker client.

First-party mobile, CLI, and MCP clients should use `/api/v1` rather than FHIR.
Those clients need MedTracker-specific workflows such as portable export/import,
sync batches, tombstones, app tokens, household administration, and capability
negotiation. FHIR is not the product sync API.

## Authorization Boundary

The current production-ready authorization boundary is MedTracker-owned bearer
credentials:

- API sessions minted by MedTracker after password login or hosted OIDC
  exchange.
- API app tokens created from an authenticated MedTracker profile session.
- Household membership and person-level grants enforced by existing policies.

For `/api/v1`, the token binds to an account, household, membership, and
permissions version. The API rejects revoked memberships, locked accounts, stale
permissions, and cross-household route use. Person-level access continues to be
derived from MedTracker policy scopes and access grants.

For `/mcp`, the same bearer boundary applies, but the exposed tool surface is
read-only.

For `/api/fhir/R4`, the current implementation uses the same MedTracker bearer
boundary and policy scopes. FHIR callers receive only records visible to the
authenticated household/person access context.

## Scope and Consent Model

MedTracker does not yet expose a third-party OAuth authorization server. It
therefore does not currently publish external OAuth scopes such as
`patient/*.read`, `launch`, or `offline_access`.

Until a third-party consent model exists, the implementation scope mapping is:

| Access concept | Current MedTracker source of truth |
| --- | --- |
| Household boundary | API session or app-token household binding |
| Person read access | `PersonPolicy` and active person access grants |
| Medication read access | medication, schedule, person-medication, and take policies |
| Admin actions | household manager role plus fresh MFA or OIDC MFA proof |
| FHIR read access | current policy-scoped FHIR controller queries |
| Sync/export access | `/api/v1` membership and grant checks |

Consent for external healthcare apps remains deferred. A future third-party
OAuth or SMART on FHIR implementation must add an explicit consent screen,
client registration, approved redirect URIs, scopes, token revocation, audit
events, and tests proving that scope grants cannot bypass household or
person-level authorization.

## SMART on FHIR

SMART on FHIR is deferred.

MedTracker supports FHIR R4 resource responses, but it does not yet support the
full SMART app launch profile. Specifically, it does not currently provide:

- third-party app registration and redirect URI governance;
- SMART launch context;
- SMART scopes and consent;
- external OAuth authorization-code issuance;
- third-party refresh-token lifecycle;
- `.well-known/smart-configuration` metadata.

The FHIR `CapabilityStatement` should describe implemented FHIR resource
behavior only. It must not imply SMART support until the SMART-specific contract
is implemented and tested.

## Rejected Alternatives

### Use `/api/v1` for all integrations

Rejected. `/api/v1` is optimized for MedTracker workflows, portable IDs, sync,
and backup semantics. External healthcare systems expect FHIR resource shapes
and FHIR error/search behavior.

### Use FHIR for first-party mobile sync

Rejected. FHIR read/search resources do not cover MedTracker's product needs
for batch mutations, tombstones, encrypted portable imports, local sync
conflicts, household administration, or client capability negotiation.

### Claim SMART on FHIR support now

Rejected. The current FHIR implementation is a read/search resource API behind
MedTracker bearer authentication. Calling it SMART-capable would overstate the
authorization, launch, consent, and scope behavior.

## Consequences

### Positive

- Client teams have a clear endpoint choice.
- FHIR docs and metadata can stay truthful and narrow.
- First-party clients can keep using MedTracker-specific sync and export
  features without forcing those concepts into FHIR.
- SMART on FHIR can be implemented deliberately instead of being implied by the
  presence of FHIR-shaped resources.

### Negative

- External healthcare apps that require SMART launch cannot integrate until the
  deferred SMART work is complete.
- MedTracker must maintain two API documentation surfaces.
- Future external clients that need both FHIR and MedTracker-specific workflows
  will need an explicit trust and onboarding decision.

## Follow-up Work

- Issue #551 remains the umbrella for OAuth API and FHIR app/LLM access history.
- Issue #1534 tracks SMART on FHIR third-party launch, consent, scopes, and
  discovery before MedTracker advertises SMART support.
- Issue #1490 tracks first-party CLI and stdio MCP tooling over `/api/v1`.
- Keep FHIR follow-up work tied to the implemented resource contract rather
  than first-party sync needs.

## Related Documents

- `docs/adrs/0005-external-identity-provider.md`
- `docs/mcp.md`
- `docs/api/openapi.v1.yaml`
- `app/controllers/api/v1/capabilities_controller.rb`
- `app/controllers/api/fhir/r4/metadata_controller.rb`
- GitHub issues #551, #1490, #1527, #1528, #1529, #1530, #1531, and #1534
