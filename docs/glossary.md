# Glossary

This glossary defines MedTracker's core domain language so model names, UI copy, and business rules stay consistent.

## Medicine inventory terms

### Remaining Supply (`medicines.current_supply`)

The number of dispensable units left **right now**.

- Decrements by 1 each time a dose is recorded.
- Drives low/out-of-stock logic.
- Should be shown in patient/carer-facing quantity displays.

### Reorder Threshold (`medicines.reorder_threshold`)

The level at or below which a medicine is considered low stock.

- `low_stock?` is true when `remaining_supply <= reorder_threshold`.

## Usage guidance

- Use **Remaining Supply** in UI copy where users need "what is left now".
- Use **Reorder Threshold** to indicate the danger zone for re-ordering.
