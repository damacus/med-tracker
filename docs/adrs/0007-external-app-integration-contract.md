# ADR 0007: External App Integration Contract

- Status
  Superseded by [ADR 0010](0010-external-integration-architecture.md)
- Date
  2026-07-09

## Context

MedTracker needed separate contracts for three types of integration:

- the MedTracker product API at `/api/v1`;
- hosted MCP at `/mcp`; and
- FHIR R4 resources at `/api/fhir/R4`.

The product API supports MedTracker workflows such as household administration,
sync, and portable data. The FHIR API supports healthcare applications that
expect standard FHIR resources and search behaviour. MCP provides read-only
tools over the MedTracker bearer credential boundary.

## Original decision

Each audience would use its own API surface. First-party clients would use the
product API. External healthcare applications would use FHIR. FHIR would not
become the product synchronisation API.

## Superseded decision

This record was written while SMART on FHIR was being delivered. Later edits
described SMART as both deferred and available. The record could no longer act
as one clear source of truth.

[ADR 0010](0010-external-integration-architecture.md) now defines the current
integration architecture, credential boundaries, and SMART App Launch support.
Use that decision for all development and operational work.
