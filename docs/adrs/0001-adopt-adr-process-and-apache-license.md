# ADR 0001: Adopt ADR practice and Apache-2.0 licensing

- Status
  Accepted
- Date
  2025-11-12

## Context

MedTracker needs a consistent record of architectural and regulatory decisions.
Contributors and deployers must be able to understand why the project chose an
approach. A permissive licence also makes adoption easier.

## Decision

1. Record architectural, regulatory, security, and product decisions as ADRs
   under `docs/adrs/`. Use sequential numbers.
2. Use the Apache License 2.0 as the project licence. Keep the repository-level
   `LICENSE` and `NOTICE` files as the public source.

## Consequences

- Contributors must use ADRs for decisions that change project architecture,
  regulation, security, or product boundaries.
- Public documentation must refer to the `LICENSE` and `NOTICE` files.
- The permissive licence should encourage NHS trusts, carers, and third parties to adopt MedTracker without legal barriers, while still requiring attribution through the NOTICE file.
- ADR numbering and format become part of the contribution guidelines and review checklist going forward.
