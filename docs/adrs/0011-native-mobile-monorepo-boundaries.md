# ADR 0011: Native Mobile Monorepo Boundaries

- Status
  Accepted
- Date
  2026-09-03

## Context

The Rails checkout currently contains an Android Gradle project at its root.
That makes Gradle treat Rails `app/` as an Android module and mixes unrelated
source ownership, build output, and local credentials. The incoming iOS
application and the planned Wear OS companion need stable boundaries before
native implementation begins.

## Decision

Rails remains at the repository root. The root owns Rails source, including
`app/`, the existing Docker and Task interfaces, and the authoritative API
contract at `docs/api/openapi.v1.yaml`. Native applications live only below
`mobile/`; the repository root is never an Android or Xcode build root.

Each native client owns an independent build root:

| Client | Build root | Delivery rule |
| --- | --- | --- |
| Android phone and Wear OS companion | `mobile/android` | Land the relocated phone application first; add Wear as a dependent, connectivity-only change. |
| iOS application | `mobile/ios` | Import only the verified, pushed source SHA with non-squashed history. |

The root OpenAPI document remains the sole authoritative Rails API description.
Each native client may check in a pinned contract copy, provenance, checksum,
and generated artifacts. A client changes its pin only through its own explicit
update task; ordinary native builds do not generate directly from live Rails
source.

Native delivery lanes remain separate from Rails CI. Android uses its nested
Gradle root and native workflow. iOS uses its nested Xcode root and native
workflow. Root Task interfaces for a native client are added only when that
client lands; this foundation adds no Task include or CI wiring for absent
applications.

## Non-goals

- Moving Rails, its `app/` directory, or its existing root build interfaces.
- Changing Rails API behaviour, database schema, authorization, deployment, or
  production delivery.
- Importing the iOS source before its verification gate, archiving its Forgejo
  repository, or squashing its history.
- Adding medication actions, direct Rails networking, or credentials to Wear
  OS.
- Moving unrelated files or build output from the original Android checkout.

## Consequences

### Positive

- Rails and native tools cannot claim each other's source directories.
- Native releases can reproduce generated clients from explicit contract pins.
- Android, Wear, and iOS can be delivered and verified independently without
  changing Rails CI requirements.

### Negative

- Maintainers must choose the correct nested build root for native work.
- Contract updates require deliberate client regeneration rather than following
  Rails changes automatically.
- The repository accepts retained iOS history and separate native workflows as
  the cost of clear ownership.

## Related documents

- [Bounded Context Map](0009-bounded-context-map.md)
- [External Integration Architecture](0010-external-integration-architecture.md)
- `mobile/README.md`
- [MedTracker OpenAPI contract](../api/openapi.v1.yaml)
