# ADR 0001: Adopt ADR practice and Apache-2.0 licensing

- Status: Accepted
- Date: 2025-11-12

## Context

MedTracker is moving toward UK health-sector compliance and broader open-source adoption. We need a consistent way to record architectural and regulatory decisions so that contributors and deployers can understand why certain approaches were taken. The new compliance plan @docs/uk-regulatory-compliance-plan.md documents a preference for a permissive licence to minimise adoption friction.

## Decision

1. All significant architectural, regulatory, security, and product decisions will be captured as Architectural Decision Records (ADRs) stored under `docs/adrs/` using sequential numbering.
2. The project will use the Apache License 2.0 as its governing licence. A repository-level `LICENSE` and `NOTICE` file will be added to formalise this decision, and future documentation (e.g., README, deployment guides) will reference the licence accordingly.

## Consequences

- Contributors must document future impactful decisions via ADRs to maintain a clear audit trail.
- We must add supporting licence artefacts (`LICENSE`, `NOTICE`) and update public documentation to reference Apache-2.0.
- The permissive licence should encourage NHS trusts, carers, and third parties to adopt MedTracker without legal barriers, while still requiring attribution through the NOTICE file.
- ADR numbering and format become part of the contribution guidelines and review checklist going forward.
