# Verification and review

For production changes, capture a failing observable-behaviour regression before
implementation, as required by AGENTS.md. A test that only searches source text
does not establish route, authorisation or form behaviour.

Choose the layer that can prove the risk:

| Risk | Evidence |
| --- | --- |
| Pure projection, Decimal calculation or date boundary | Rust unit/integration test with controlled inputs and clock |
| Request parsing, access, transaction or response compatibility | HTTP contract test using the disposable PostgreSQL fixture |
| User submission, redirects, field errors, locale or session lifecycle | Playwright against the Rust listener |
| Visual fidelity and mobile usability | Desktop/mobile screenshots plus browser interaction |
| Trigger, RLS or migrated-schema dependency | Migrated database check, beyond schema-only fixture coverage |

Start with the affected case and current task wrappers. Expand checks when the
change crosses shared boundaries or repository gates require it. Do not run the
Rails suite for a Rust-only change merely because Rails supplies fixtures.

Report which listener each acceptance run exercised. A Rails baseline is useful
evidence for the reference behaviour, but cannot establish Rust completion.
Likewise, a compiling contract test has not exercised its HTTP assertions.

For review, prioritise access leakage, clinical effects, concurrency and parity
over Rust style. Inspect the changed path and its regression evidence; load
another reference only for a concrete unresolved concern. Acceptance counts
and route inventories do not replace a completed browser journey.
