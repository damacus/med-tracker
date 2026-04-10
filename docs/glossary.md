# Glossary

This glossary defines MedTracker's core domain language so model names, UI copy, and business rules stay consistent.

UI components and presenters must use these domain terms and consume server-side domain objects rather than inventing parallel terminology.

## Medication inventory terms

### Remaining Supply (`medications.current_supply`)

The number of dispensable units left **right now**.

- Decrements by 1 each time a dose is recorded.
- Drives low/out-of-stock logic.
- Should be shown in patient/carer-facing quantity displays.
- Prefer **Remaining Supply** or **units remaining** in new UI copy over generic **Stock** where the meaning is "what is left now".

### Supply at Last Restock (`medications.supply_at_last_restock`)

The value of `current_supply` immediately after the most recent restock.

- Set automatically by `Medication#restock!`.
- Used as the denominator for progress bars so the bar drains proportionally from 100% → 0%.
- Falls back to `reorder_threshold` when nil (e.g. legacy data before this column existed).

### Reorder Threshold (`medications.reorder_threshold`)

The level at or below which a medication is considered low stock.

- `low_stock?` is true when `remaining_supply <= reorder_threshold`.

## Medication administration terms

### Medication

The aggregate root for dosage options, supply attributes, and administration sources.

- `Medication` remains the canonical term in code for this aggregate.
- Do not introduce parallel model names for the same concept in this pass.

### Schedule

A time-based medication regimen tied to a person.

- Schedules remain distinct from ad-hoc medications.
- A `Schedule` is one source of medication administration.

### PersonMedication

An ad-hoc medication assignment outside the scheduled regimen flow.

- `PersonMedication` remains distinct from `Schedule`.
- It is the second source of medication administration.

### MedicationTake

The persisted dose record for a single administration event.

- `MedicationTake` remains the canonical persistence term in this pass.
- UI copy may use "dose" or "administration" where clearer for users, but code should not introduce a second model name.

## Usage guidance

- Use **Remaining Supply** in UI copy where users need "what is left now".
- Use **Reorder Threshold** to indicate the danger zone for re-ordering.
- Progress bars use `current_supply / supply_at_last_restock` for a proportional drain.
