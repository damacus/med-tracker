# Glossary

This glossary defines MedTracker's core domain language so model names, UI copy, and business rules stay consistent.

## Medicine inventory terms

### Remaining Supply (`medicines.current_supply`)

The number of dispensable units left **right now**.

- Decrements by 1 each time a dose is recorded.
- Drives low/out-of-stock logic.
- Should be shown in patient/carer-facing quantity displays.

### Total Supply (`medicines.stock`)

The reference total inventory level for a medicine.

- Does not decrement when a dose is recorded.
- Used as a baseline/denominator for progress and inventory ratio views.

### Reorder Threshold (`medicines.reorder_threshold`)

The level at or below which a medicine is considered low stock.

- `low_stock?` is true when `remaining_supply <= reorder_threshold`.

## Usage guidance

- Prefer **Remaining Supply** in UI copy where users need “what is left now”.
- Keep **Total Supply** as the term for baseline total/reference level.
- If both are displayed together, format as `remaining_supply / stock`.
