# Native mobile workspace

`mobile/` is the only home for native MedTracker applications. Rails remains
at the repository root; `app/`, the root Task interface, Docker configuration,
and `docs/api/openapi.v1.yaml` remain Rails-owned.

## Build roots

| Client | Build root | Status |
| --- | --- | --- |
| Android phone and Wear OS companion | `mobile/android` | Reserved for the verified Android relocation. |
| iOS application | `mobile/ios` | Reserved for the verified full-history iOS import. |

Open Android from `mobile/android` and iOS from `mobile/ios` after those
projects land. Do not open the repository root as an Android Gradle or Xcode
project. Native caches, derived output, local SDK paths, and result bundles
belong to their native build roots and remain untracked.

## API contracts

`docs/api/openapi.v1.yaml` is the authoritative MedTracker API contract. A
native client may retain a pinned OpenAPI copy with provenance, checksum, and
generated artifacts. Each client adopts a new contract deliberately through
its own update task; normal native builds use the checked-in pin.

## Delivery lanes

1. This foundation records the boundaries only.
2. Android relocates the phone application, then adds release hardening and
   the connectivity-only Wear OS companion.
3. iOS imports independently only after the source task verifies a clean,
   pushed SHA and complete native checks.

Native workflows remain separate from Rails CI. Root Android or iOS Task
commands are introduced only with their corresponding landed application.

## Non-goals

- Moving Rails or changing its API, database, authorization, deployment, or
  existing root build interface.
- Treating Wear OS as a second Rails client or storing Rails credentials on it.
- Importing unverified iOS work, archiving the Forgejo source repository, or
  copying unrelated Android build output and local configuration.
