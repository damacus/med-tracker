# Glossary

This glossary defines MedTracker's core domain language so model names, UI copy, and business rules stay consistent.

UI components and presenters must use these domain terms and consume server-side
domain objects rather than inventing parallel terminology.

Prefer these names in new code, UI copy, documentation, and tests. Avoid introducing new patient, individual, plan, routine, stock, or dose-record names unless the surrounding bounded context already requires them.

## People and care terms

### Person

The individual receiving medication or being represented in MedTracker.

- `Person` is the canonical model and code term for the tracked individual.
- User-facing copy may say "person" or a more specific relationship label when context is clearer.
- Avoid introducing parallel `Patient`, `Individual`, or `Subject` abstractions for this concept.

### Carer

A person with delegated responsibility for another person.

- Carer relationships grant scoped access to the dependent person's medication records.
- Use `CarerRelationship` for the relationship record.
- Use "carer", "parent", or another explicit relationship label in UI copy when
  that is clearer than a generic user role.

## Medication inventory terms

### Supply

The inventory quantity for a medication.

- Use `Supply` as the umbrella domain term for inventory quantity.
- Use the more specific terms below when describing current quantity, last restock quantity, or reorder thresholds.
- Prefer "Remaining Supply" over "Stock" for new patient/carer-facing copy when the meaning is the amount left now.

### Remaining Supply (`medications.current_supply`)

The number of dispensable units left **right now**.

- Decrements by 1 each time a dose is recorded.
- Drives low/out-of-stock logic.
- Should be shown in patient/carer-facing quantity displays.
- Prefer **Remaining Supply** or **units remaining** in new UI copy over generic
  **Stock** where the meaning is "what is left now".

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
- Avoid introducing a separate `DoseRecord` model name unless a future ADR changes the persistence boundary.

## Usage guidance

- Use **Remaining Supply** in UI copy where users need "what is left now".
- Use **Reorder Threshold** to indicate the danger zone for re-ordering.
- Progress bars use `current_supply / supply_at_last_restock` for a proportional drain.
- Use **Schedule** for time-based medication regimens, not **Plan** or **Routine** in new code.
- Use **MedicationTake** for persisted administration records, not **DoseRecord** in new code.
