# Existing stock workflow verification

Verify listMedicationStockRemovals, createMedicationStockRemoval,
adjustMedicationInventory, markMedicationAsOrdered and markMedicationAsReceived
against the fixed OpenAPI contract. Existing handlers receive no completion
credit until strict contract tests and the relevant existing stock/security
tests pass against Rust.

Reject unknown fields, invalid scalar types and forbidden nulls without stock
or history changes. Preserve decimal precision, tenant and role boundaries,
stock-history response shapes, audit and idempotency behavior. Validate the
documented page and per_page limits rather than silently clamping them.

The received operation has no request body in OpenAPI: accept a bodyless PATCH.
Preserve order/receipt transitions, inventory adjustment semantics and exact
stock changes. Reuse concurrent stock-removal proof that stock cannot be
overdrawn and retries cannot duplicate writes.

Document missing 422 responses for constrained pagination and strict order
inputs as response-contract clarifications. Do not add operation IDs or change
the fixed 118-operation inventory. Compile focused tests, demonstrate failing
behavior, then let Sol fix the existing handlers while Luna completes tests.
Independent review, isolated acceptance and publication precede ledger credit.
