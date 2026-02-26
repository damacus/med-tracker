# Glossary

This glossary defines MedTracker's core domain language so model names, UI copy, and business rules stay consistent.

## Medication inventory terms

### Remaining Supply (`medications.current_supply`)

The number of dispensable units left **right now**.

- Decrements by 1 each time a dose is recorded.
- Drives low/out-of-stock logic.
- Should be shown in patient/carer-facing quantity displays.

### Supply at Last Restock (`medications.supply_at_last_restock`)

The value of `current_supply` immediately after the most recent restock.

- Set automatically by `Medication#restock!`.
- Used as the denominator for progress bars so the bar drains proportionally from 100% → 0%.
- Falls back to `reorder_threshold` when nil (e.g. legacy data before this column existed).

### Reorder Threshold (`medications.reorder_threshold`)

The level at or below which a medication is considered low stock.

- `low_stock?` is true when `remaining_supply <= reorder_threshold`.

## Usage guidance

- Use **Remaining Supply** in UI copy where users need "what is left now".
- Use **Reorder Threshold** to indicate the danger zone for re-ordering.
- Progress bars use `current_supply / supply_at_last_restock` for a proportional drain.
