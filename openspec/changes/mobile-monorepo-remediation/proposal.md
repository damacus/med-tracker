## Why

The Rails checkout currently contains an Android Gradle project at its root, causing Gradle to treat Rails `app/` as an Android module and mixing unrelated build outputs, credentials, and source ownership. The incoming iOS application and planned Wear OS companion require explicit monorepo boundaries before further native development. Origin: [#2039](https://github.com/damacus/med-tracker/issues/2039).

## What Changes

- Keep the Rails application at the repository root and place native applications under `mobile/`.
- Relocate the Android phone application into a standalone `mobile/android` Gradle project.
- Harden Android authentication, credential storage, backup, logging, live-test isolation, and generated API-contract handling.
- Add a connectivity-only Wear OS companion that communicates through the paired Android phone.
- Import the completed iOS application under `mobile/ios` with its full Git history and native CI workflow.
- Keep the root OpenAPI document authoritative while allowing each native client to pin and regenerate its own versioned contract artifacts.
- Add root Task interfaces and separate native CI workflows without changing Rails API behaviour.

## Capabilities

### New Capabilities

- `native-mobile-workspace`: Repository layout, root task interfaces, build isolation, and native CI boundaries.
- `native-client-contract-pinning`: Per-client OpenAPI pinning, provenance, generation, and drift validation.
- `android-client-security`: Release authentication, credential storage, backup, logging, and canary-test boundaries for the Android phone app.
- `wear-companion-connectivity`: Paired-phone discovery and persistent companion status for the connectivity-only Wear OS application.
- `ios-history-import`: Full-history iOS subtree import, provenance, and GitHub-hosted Xcode verification.

### Modified Capabilities

None. Rails API behaviour and existing domain requirements remain unchanged.

## Impact

- Affects repository documentation, Taskfile wiring, GitHub Actions, native application source, and generated client artifacts.
- Adds Android Wearable Data Layer and Android secure-storage dependencies within `mobile/android`.
- Imports the existing iOS repository and its history without archiving or deleting the Forgejo source repository.
- Does not change Rails routes, database schema, authentication contracts, production deployment, or unrelated working-tree changes.
