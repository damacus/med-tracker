## Purpose

Brings the completed MedTracker iOS application into the monorepo while preserving its source history, reproducible API package, and native verification contract.

## ADDED Requirements

### Requirement: iOS import is gated by a verified immutable source
The iOS application SHALL be imported only after its source task has completed, its full CI task is green, its working tree is clean, its commits have the expected identity and signing, and its final branch is pushed with an exact recorded commit SHA.

#### Scenario: Source build is incomplete
- **WHEN** the source iOS task is active, unpushed, dirty, unsigned, or failing verification
- **THEN** no iOS subtree import is performed

### Requirement: iOS source history remains reachable
The monorepo SHALL import the exact verified iOS source commit without squashing and SHALL record source repository, branch, commit, and import date provenance.

#### Scenario: History verification
- **WHEN** the import merge is inspected
- **THEN** the recorded source commit is its ancestor, remains inspectable, and the imported file manifest matches that source tree

### Requirement: Imported iOS CI runs on GitHub-hosted macOS
The monorepo SHALL run the imported iOS CI task using GitHub-hosted macOS 26 with Xcode 26.6 explicitly selected and verified.

#### Scenario: iOS pull request
- **WHEN** imported iOS source or its native workflow changes
- **THEN** GitHub Actions runs the complete iOS CI task and retains diagnostic result bundles on failure
