# Design

## Context

See proposal.md for motivation. `src/models/care/ai_suggestions.rs` owns feature/provider configuration, authorisation and generation. `src/controllers/api/care/ai_suggestions.rs` currently authorises, rolls back its transaction, calls generation, then opens another transaction for audit. Generation can call a provider more than once and fetch public sources. Browser household settings already exist in `src/controllers/administration.rs`; API settings use `src/controllers/api/care/administration.rs`.

## Goals / Non-Goals

**Goals:** a persisted, auditable household decision; fail-closed outbound enforcement; proof through actual application acceptance and a counting fake provider.

**Non-Goals:** new provider credentials, consent inherited through portability, provider transport redesign beyond the enforcement seam, or a replacement authorisation system.

## Decisions

1. Add a typed household boolean with database default false and non-null semantics, using the repository's forward-only SeaORM migration and entity patterns. Existing rows stay false. Avoid a loosely typed preference or implicit plan-derived consent. Keep imports from granting consent by omission or copying it from another household.
2. Reuse household administration operations and existing owner/administrator checks. Expose the value in the existing settings form and applicable API DTOs/allowlists; preserve omitted fields on unrelated API updates. Add translated disclosure explaining that submitted medication details go to the configured external provider and that withdrawal stops future sends. Audit actual changes using the existing household audit transaction, without medication content or credentials.
3. Keep the instance feature flag, family_plus eligibility and credential scopes. Add consent to shared model authorisation so browser/API entry points cannot bypass it. Return the existing unavailable/disabled feature response where possible and update OpenAPI deliberately if required.
4. Authorisation before the controller releases its transaction is insufficient for the strict send guarantee. Use database coordination shared by consent updates and outbound work: freshly authorise under a household row lock before each outbound send and retain the lock through that send, or an equally strong existing repository mechanism. Consent withdrawal must obtain a conflicting lock before committing. Recheck after lock waits. Use shared read locking for sends where supported to avoid unnecessarily serialising readers. Do not rely on a process-local mutex or an unchecked cached boolean.
5. Keep outbound timeouts and limits. Do not hold a database lock while doing unrelated parsing or rendering. If a send is already executing when withdrawal starts, withdrawal waits for that send; after withdrawal succeeds, no subsequent send starts. Apply the same consent boundary to every AI provider call, including candidate discovery and final synthesis. Reject before public-source fetching when no consent exists.
6. Verify through the full Loco test application with disposable PostgreSQL, restricted runtime credentials and synthetic data. A local counting fake provider must observe zero requests for default-off, disabled, withdrawn, cross-household and revoked-authority cases; retain a positive opted-in control. Add a deterministic race test around withdrawal and a later send. Observe failing tests before executable changes.

## Risks / Trade-offs

- Network calls while holding a row lock increase database occupancy and can delay settings writes → use existing short HTTP timeouts, narrow per-send transactions and deterministic concurrency coverage. Do not add a custom distributed lock system.
- Settings serializers or import paths could accidentally reset or enable consent → exercise omitted-field updates, new-household creation and relevant imports; never copy consent as an implied entitlement.
- Changing shared fixtures can conceal the default-off regression → opt in only AI-positive fixtures and separately assert default false.
- The checkout trails fetched main → integrate current main safely before implementation while preserving these artifacts and unrelated work; recheck new migrations for numbering conflicts.

## Migration Plan

Add a forward migration and fresh-schema support without resetting or adopting databases. Verify existing and newly created households remain off. Ship UI, enforcement and contract updates as one coherent slice; do not deploy from this task. If the feature must be disabled operationally, use its existing instance-wide flag rather than reverting consent enforcement.
