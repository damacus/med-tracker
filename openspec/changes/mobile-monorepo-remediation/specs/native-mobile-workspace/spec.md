## Purpose

Defines stable repository and build boundaries so Rails and each native client can be developed, tested, and released without claiming one another's source or generated files.

## ADDED Requirements

### Requirement: Native applications have isolated build roots
The repository SHALL keep Rails at the repository root, keep `app/` exclusively owned by Rails, and place native clients beneath `mobile/` with independent Android and iOS build roots.

#### Scenario: Android project discovery
- **WHEN** a developer opens `mobile/android` in Android Studio or runs its Gradle wrapper
- **THEN** Gradle discovers the phone, Wear, and shared protocol modules without treating the Rails `app/` directory as a module

#### Scenario: Rails project discovery
- **WHEN** a developer runs the existing Rails task interface from the repository root
- **THEN** Rails uses its existing root paths without depending on a native build tool

### Requirement: Root tasks expose native verification
The root task interface SHALL provide stable Android and iOS verification commands while native implementation details remain in each native build root.

#### Scenario: Android verification
- **WHEN** a developer runs `task android:ci`
- **THEN** the Android project runs compilation, lint, tests, release checks, and contract drift validation

#### Scenario: iOS verification
- **WHEN** a developer runs `task ios:ci`
- **THEN** the imported iOS project's complete CI task runs from `mobile/ios`

### Requirement: Native generated files remain untracked
Native builds SHALL ignore caches, derived output, local SDK paths, result bundles, and other machine-specific files without ignoring Rails source.

#### Scenario: Clean build output
- **WHEN** Android and iOS builds complete
- **THEN** `git status` reports no generated or machine-specific native files as untracked changes
