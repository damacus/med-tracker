## Context

See `proposal.md` for motivation. Rails already owns a mature household-scoped `/api/v1` surface and `docs/api/openapi.v1.yaml`. The untracked Android scaffold currently places its Gradle root beside Rails and names Rails `app/` as its Android module. A separate Forgejo iOS repository is under active implementation and must not be imported until its verification gate completes.

## Goals / Non-Goals

**Goals:**

- Give Rails, Android phone, Wear OS, and iOS unambiguous source and build ownership.
- Preserve native-client release reproducibility without coupling clients directly to live Rails models.
- Remove credential, backup, logging, and live-test hazards from the Android client.
- Preserve the complete iOS source history and existing native verification interface.

**Non-Goals:**

- Rails API, database, authorization, or deployment changes.
- Medication functions on Wear OS, watchOS, application-store delivery, or iOS source-repository archival.
- Moving or including unrelated edits from the original dirty checkout.

## Decisions

### Rails remains at the repository root

Rails conventions and the existing Docker, Task, CI, and deployment paths remain unchanged. Native clients live under `mobile/`; moving Rails under another directory would create broad risk without solving a runtime boundary. The rejected alternative is a multi-repository mobile layout because the approved direction is a single monorepo with one authoritative API contract.

### Android becomes an independent nested Gradle project

`mobile/android/settings.gradle.kts` initially includes `:phone`, then the Wear change adds `:wear` and `:wear-protocol`. Only source, wrapper, and configuration are copied from the untracked scaffold. Caches, build output, and `local.properties` are never migrated. The original files remain until the pushed relocation is verified by checksums.

### Native contract pins are explicit release inputs

The root OpenAPI document remains authoritative, but each client checks in a pinned copy, provenance, checksum, and generated code. Update tasks make contract adoption deliberate; check tasks regenerate in temporary output and fail on drift. Direct generation from live Rails source on every native build was rejected because it would prevent independent client releases.

### Android release security is compile-time enforced

Release exposes OIDC PKCE only and uses a fixed server configuration. Debug and staging alone may compile password login and server overrides. Tokens are encrypted with a key held by Android Keystore and encrypted bytes live under `noBackupFilesDir`; Android backup is disabled. Release logging is `NONE`; opt-in non-release logging is `BASIC` with sensitive headers redacted. A runtime flag alone was rejected because release binaries must not contain the password-login interface.

### Wear is a phone companion, not a second mobile client

The phone advertises `medtracker_phone_companion_v1` through `CapabilityClient` and writes the small, persistent `/medtracker/companion/status` DataItem. `wear-protocol` owns versioned paths, enums, and deterministic JSON encoding shared by phone and watch. Wear derives connection state from discovery plus the latest status and displays only connectivity states. `MessageClient`, medication actions, direct network access, and credentials are deferred until a separately specified capability needs them.

### iOS is imported with a non-squashed subtree merge

After the source gate passes, an exact pushed SHA is imported at `mobile/ios`. The mechanical import commit is followed by a distinct integration commit for root tasks, provenance, ignores, and GitHub Actions. The source SHA must remain an ancestor of the merge. Snapshot copying and squashed subtree import were rejected because both lose the requested history.

### Native CI remains separate from Rails CI

Android runs on Ubuntu with JDK 17. iOS runs on GitHub-hosted `macos-26` with Xcode 26.6 selected and verified. Existing Rails CI remains unchanged until required-check rules are separately audited. Native workflows use path filters for their own trees and retain diagnostic artifacts on failure.

## Risks / Trade-offs

- [Untracked Android files change during migration] → Record source and destination manifests and retain the original until the pushed branch is green.
- [Android security rewrite changes login behaviour] → Implement observable security properties test-first and keep staging password login explicitly separate.
- [Wear Data Layer availability varies by device] → Model unavailable, disconnected, incompatible, signed-out, and ready states and test them behind adapters.
- [iOS source task remains incomplete] → Continue other delivery lanes and keep the import task blocked; never import an unverified intermediate commit.
- [Full-history import makes the repository larger] → Accept the cost because history preservation is an explicit requirement and retain immutable provenance.
- [Native CI consumes substantial hosted capacity] → Use path-filtered native workflows while leaving Rails required checks unchanged.

## Migration Plan

1. Land the documentation and boundary foundation.
2. Relocate the Android phone app from a verified source manifest, then harden it and add Wear as dependent changes.
3. Import iOS independently after its source gate succeeds.
4. Verify each branch through its root Task interface and publish dependent pull requests without merging or deploying them.
5. Retain original Android files and the Forgejo iOS repository until their replacements are independently verified; cleanup and archival require separate approval.
