# Stock adjustment with dosage options

Independent bounded source ruling, 1 October 2026. No runtime, fixture mutation,
API call or source/test edit was performed. The coordinator owns acceptance.

## Existing parent adjustment does not select an option

`PATCH /households/{household_id}/medications/{id}/adjust_inventory` accepts only
`adjustment.new_quantity` and optional `reason`: authoritative
`docs/api/openapi.v1.yaml:6525` declares additionalProperties false and no
`dosage_id`. Rust `medication_management/inventory.rs:117` rejects any other
attribute. Rails `api/v1/medications_controller.rb:42` forwards only quantity and
reason to `AdjustMedicationInventoryService`.

Both implementations update the parent quantity without changing dosage rows:
Rust `inventory.rs:168` locks the medication, reloads it, then sets only
`current_supply` and `updated_at`; Rails
`adjust_medication_inventory_service.rb:18` uses a medication lock and updates
only parent `current_supply`. Neither selects/reconciles tracked option stock.
An absolute parent adjustment with tracked options therefore leaves the parent
different from their sum; a subsequent option inventory change can overwrite it
with the aggregate. This is supported parent-only API behaviour, not a safe
per-option adjustment contract. Do not invent a `dosage_id` parameter or promise
that parent adjustment redistributes quantities across options.

## Supported absolute per-option stock update

Use the existing `PATCH /households/{household_id}/dosage_options/{id}` with
`dosage_option.current_supply`, optionally `reorder_threshold`, and the original
option If-Match. OpenAPI `:1868` explicitly describes atomic parent aggregation,
nullable stock, and stale supplied token 409; `:6726` exposes nullable decimal
stock fields. No new endpoint is needed.

Rust `dosage_options/update.rs:40` locks household then parent medication, then
reloads the option under exclusive lock. `:54` compares a supplied original
If-Match; the API remains optional-token compatible while browser edits require
the captured token. `:87` detects inventory changes; `:126` saves within a
savepoint and `:144` calls aggregate synchronisation before commit. In
`dosage_options/inventory.rs:18`, tracked rows alone contribute quantity and
threshold; no tracked rows resets parent stock/baseline to null and threshold
to zero. Null is untracked and zero is tracked empty stock. `update.rs:163–207`
versions and syncs the option and changed parent under a common request ID;
the shared finish audits and commits the encompassing transaction. This is
absolute edit concurrency, not create replay/idempotency protection.

Rails exposes the same stock fields through
`api/v1/dosage_options_controller.rb:28–35`. `MedicationDosageOption` has
PaperTrail at `app/models/medication_dosage_option.rb:8` and an after-commit
inventory callback at `:15`; `:98–108` triggers it for tracked/change-to-null
stock. Parent aggregation is `app/models/medication.rb:81–90`. Rails source
supports the per-option path, but its after-commit reconciliation is not evidence
that it shares Rust's enclosing atomic transaction. No Rails concurrency run was
performed here.

For an actual stock-removal event, the existing removal API separately supports
`dosage_id`, quantity/reason and `submission_id`: OpenAPI `:7989` and Rust
`stock_removals.rs:364–447` scope/lock the selected option and update its stock.
The removal replay check at `:384` preserves exact repeated-payload semantics.
Do not use a removal to invent an absolute increase or restock operation.

## Truthful E interface

For scalar medication stock, expose the existing absolute adjustment operation.
When tracked dosage options exist, show their quantities and link/select the
real option stock edit; explain that parent quantity is their calculated total.
Preserve original option preconditions and no-write stale/foreign/permission
checks. Do not offer a parent-total field that implies reconciled option stock.
If all options are untracked, setting stock on an actual option through its edit
form is the coherent route to tracked option inventory; any additional parent
tracking presentation needs an explicit verified contract ruling.

## Existing audit parity gap

Rust parent adjustment currently validates `reason` but never persists it in its
version payload/event: `inventory.rs:124` checks type, while `:176–187` records
the literal event `adjust inventory` with medication snapshots only. Rails
`adjust_medication_inventory_service.rb:16` and `:30–33` includes quantity and
reason in the PaperTrail event. Recommended follow-up labels: `rust`, `bug`.
Do not promise that a Rust adjustment reason was recorded until a bounded
RED/GREEN-backed compatibility fix proves it. Per-option PATCH has no adjustment
reason field; its existing audit evidence is before/after option and parent
snapshots, not an invented stock-event reason.

Final E acceptance requires matching-source actual stock/aggregate, concurrent
edit, audit/sync and replay receipts. Source inspection alone is not acceptance.
