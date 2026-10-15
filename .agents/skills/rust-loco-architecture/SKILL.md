---
name: rust-loco-architecture
description: >-
  Enforces typed domain architecture, SeaORM/RLS safety, Tera view conventions, and error observability when writing or refactoring Rust/Loco code in MedTracker. Use whenever implementing features, endpoints, models, workers, or refactoring in `src/`.
---

# rust-loco-architecture

Read root AGENTS.md and ADRs 0013–0016 before applying this skill.
These are accepted direction, not evidence that pending application work passes.

- HTTP/Form Input -> Request DTO -> Validated Domain Command -> Operation ->
  Typed Result -> Response DTO. Never serialize a struct into serde_json::Value
  to pass internal module boundaries, including execute_in_timezone/dose writes.
- Distinct AccountId, PersonId, MedicationId, HouseholdId, ScheduleId, AssignmentId
  newtypes; enum DoseSource { Schedule(ScheduleId), Assignment(AssignmentId) };
  boundary parsing of timestamps with explicit local timezone context and
  Decimal amounts; no f32/f64 or raw strings for dose math.
- Loco AppContext owns boot, queues, storage/OpenDAL, settings; HTTP/storage
  transports follow operator lifecycles, not process-global ephemeral runtimes.
- Better Auth RS handles authentication; household tenancy, care disclosure,
  revocation rechecks and RLS must have one explicit application authority.
  The initial 'strictly authentication only' rule was challenged and superseded:
  do not require removing its organisation plugin without evidence. Keep current
  atomic adapters where the plugin's separate writes cannot meet atomicity.
  No legacy account -> person -> user chain; accounts contain profile fields.
- OperationError/From<DbErr> clients receive safe generic errors, while internal
  structured traces retain categories (constraint/timeout/connection) and trace
  IDs without clinical values, raw SQL binds or full raw database messages.
- Root views are assets/views Tera and DaisyUI, never Ruby/Phlex.

Use Fish/RTK and Task entry points. Apply TDD to executable changes.
No broad lint suppressions, including tests or generated entities.
Keep Rails read-only; no legacy data/credential import, parity or reverse migration.
Never reset or silently adopt a populated database. Preserve API/MCP contracts
except explicit access/location changes documented in OpenAPI, clients and ADRs.
