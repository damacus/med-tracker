## Task 1: Establish monorepo boundaries

- [x] 1.1 Add an accepted ADR and `mobile/README.md` defining Rails-root ownership, native build roots, contract authority, delivery lanes, and explicit non-goals; verify with `task docs:build` and `git diff --check`.
- [x] 1.2 Update `AGENTS.md` and `agents.md` together with concise Android and iOS routing rules while preserving all existing Rails instructions; verify the files remain synchronized.
- [x] 1.3 Validate the complete `mobile-monorepo-remediation` OpenSpec change with strict validation and commit the foundation using the verified Dan Webb identity and signing configuration.

## Task 2: Relocate the Android phone application

- [x] 2.1 Record a deterministic manifest and SHA-256 checksums for the intended untracked Android source, wrapper, and configuration in the original checkout while excluding `.gradle`, all `build/` directories, and `local.properties`; retain the originals until the pushed relocation is verified.
- [x] 2.2 Copy the verified Android inputs to `mobile/android`, rename the application module from `app` to `phone`, and update Gradle settings so the Rails root contains no Android build root and Android Studio discovers only `:phone`; verify source and destination manifests match.
- [x] 2.3 Add scoped Android ignores, a nested Android Taskfile, root `android:test`, `android:api:check`, `android:api:update`, and `android:ci` commands, plus an Ubuntu/JDK 17 path-filtered workflow; verify the focused Gradle tests, lint, build, contract check, and `git status --ignored`.
- [x] 2.4 Add a pinned Android OpenAPI copy, provenance and checksum plus deterministic generated transport artifacts behind the app-owned API interface; verify an intentional generated-file drift causes `task android:api:check` to fail before restoring green state.

## Task 3: Harden Android release boundaries

- [x] 3.1 Add failing tests for release-only OIDC PKCE, absence of password/server override UI in release, and staging-only password/server configuration; implement the minimum build and UI changes and verify those tests pass.
- [x] 3.2 Add failing tests proving tokens never enter ordinary preferences or backup-eligible storage and backup is disabled; implement Keystore-backed encrypted credentials under no-backup storage and verify the security tests pass.
- [x] 3.3 Add failing tests for release logging `NONE`, opt-in non-release `BASIC` logging, redacted authorization/session headers, and absent body logging; implement the logging policy and verify tests pass.
- [x] 3.4 Move the canary test to a dedicated opt-in integration task with environment credentials and prove ordinary tests and pull-request CI neither select it nor contact canary.
- [x] 3.5 Run `task android:ci`, root configuration checks, `task rubocop`, and `task test`; commit only when all applicable gates pass with a clean tracked tree.

## Task 4: Add the Wear OS connectivity foundation

- [x] 4.1 Add failing protocol tests for `medtracker_phone_companion_v1`, `/medtracker/companion/status`, deterministic versioned payload round-trips, and unsupported versions; implement the small shared `wear-protocol` Kotlin module and verify tests pass.
- [x] 4.2 Add failing phone tests for capability advertisement and persistent companion status publication with protocol version, phone app version, session state, and publication time; implement the Data Layer adapter and verify tests pass.
- [x] 4.3 Add failing Wear tests for phone-app-missing, disconnected, incompatible, signed-out, ready, and reconnect convergence; implement the Wear application and minimal connectivity UI using capability discovery and persistent data only.
- [x] 4.4 Add static checks proving Wear contains no Rails URL, credentials, password flow, generated Rails client, or direct network transport; add a paired phone/watch emulator smoke task and run it where an emulator pair is available.
- [x] 4.5 Run `task android:ci` and verify Android Studio/Gradle discovers `:phone`, `:wear`, and `:wear-protocol`; commit only when generated files remain ignored and the tracked tree is clean.

Task 4 verification note: Android CI passed with 51 tests, both lints, six APKs, release security, Wear boundaries and generated-contract drift checks. Three-module discovery, repository gates and signed commits are verified. Whole-lane review corrections isolate dashboard/profile state and bearer headers across sessions, with behavioural and wire-level regression coverage. The paired-emulator smoke is runnable but remains unverified because no phone/watch emulator pair was available.

## Task 5: Import the verified iOS application

- [ ] 5.1 Wait until the “Build MedTracker iOS app” task is complete, then verify its source tree is clean, `task ci` and required build/unit/UI/API/archive gates are green, commits use the expected identity/signing, and the final branch is pushed with an exact immutable SHA; do not import if any gate is unmet.
- [ ] 5.2 From a clean branch based on the foundation commit, import the exact iOS SHA at `mobile/ios` using a signed non-squashed subtree merge and record source repository, branch, commit, and import date provenance.
- [ ] 5.3 Verify the source SHA is an ancestor of the import merge, remains inspectable, and the imported file manifest matches the source tree; keep the mechanical import and integration work in separate commits.
- [ ] 5.4 Add root `ios:ci`, `ios:api:check`, and `ios:api:update` commands, preserve the pinned generated API package, and add a path-filtered GitHub workflow using hosted macOS 26 with Xcode 26.6 explicitly selected and failure result bundles retained.
- [ ] 5.5 Run the imported `task ios:ci`, relevant root checks, and history/provenance verification; retain the Forgejo repository unchanged and publish the iOS import as a separate pull request based only on the foundation change.
