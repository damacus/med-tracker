# Proposal

## Why

AI medication suggestions can currently send submitted medication details to an instance-configured provider without household consent. The owner requires explicit household opt-in, defaulting to off, and proof that requests without consent send no data.

Origin: owner instruction in Codex chat `01a1ccd6-4225-7402-acff-7fe7e12535fe`, following the 11 November 2026 security review. No originating GitHub issue was supplied.

## What Changes

- Persist household AI consent with a default of false for existing and new households.
- Allow existing household managers (owners and administrators) to grant or withdraw consent through household settings, with clear disclosure and attributable audit.
- Require current consent before outbound AI suggestion work, in addition to current membership, feature, plan and credential checks.
- Verify zero outbound requests when consent is absent, false or withdrawn, including cross-household and unauthorised changes.
- **BREAKING**: previously eligible households cannot use AI suggestions until a manager explicitly opts in.

## Capabilities

### New Capabilities
- `household-ai-consent`: household-controlled permission for sending medication suggestion inputs to an external AI provider.

### Modified Capabilities
None.

## Impact

Household persistence and migrations, administration settings UI/API, AI authorisation and outbound orchestration, audit, OpenAPI and focused acceptance/browser tests. Reuse current Loco/SeaORM/Tera boundaries and instance credentials; no new dependency is expected.

Non-goals: per-household provider keys, provider selection redesign, billing changes, person-level consent products, mobile UI redesign, production deployment or live-data changes.
