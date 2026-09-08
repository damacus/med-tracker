## Context

See proposal.md for motivation. Inventory lives on Medication or its dosage records. Audit::VersionEvent already records structured, attributable workflow evidence. MedicationTakeStockDecrement locks dosage stock before synchronising the parent medicine.

## Goals / Non-Goals

Provide an online web workflow without changing dose semantics, stock tracking policy or permission levels. Do not introduce a parallel stock ledger or new portable-data format.

## Decisions

- Add an inventory service with a distinct `stock_removal` audit event under item type `MedicationStockRemoval`, using the medicine ID as item ID. The structured event contains quantity, source ID, unit, before/after quantities, reason, optional note and submission UUID. This avoids pretending an event snapshot is a reifiable Medication version. Existing audit context records the actor and time.
- Reuse the existing inventory update permission. Resolve the medicine through household policy scope and any dosage source through that medicine. Never accept an arbitrary stock record.
- Lock a selected dosage before the medicine, matching dose consumption ordering. Perform the decrement, parent synchronisation and event insert in one transaction. Suppress deferred dosage inventory synchronisation and synchronise within the transaction.
- Accept positive finite decimals exactly representable at the stored two-decimal precision. Reject missing stock, excessive precision, insufficient stock, unknown reasons and notes over 1,000 characters. Show a catalogue stock unit only when it represents inventory units; otherwise label it units.
- Generate a UUID per form. Under the medicine lock, query previous removal evidence for that medicine, household and UUID. Identical payloads succeed without another decrement; changed payloads fail. Audit retention therefore also bounds replay protection; the online form is not a permanent offline command.
- Add a dedicated GET/POST web form linked from the medicine page, with recent removal history loaded by the controller. Render validation errors with entered values and use a 303 redirect on success. Use standard accessible components, labels and translated text. Filter the stock-removal parameter group from request logs; free text appears only in authorised form/history and audit evidence.
- Keep absolute corrections unchanged. Rejected alternatives: treating removals as takes corrupts administration history; converting to an absolute count loses intent and invites stale-write errors; a new domain table would unnecessarily expand tenancy, purge and portability work for this bounded workflow.

## Risks / Trade-offs

- Multiple tracked dosage records → require an explicit stock choice; never subtract only from an aggregate.
- Concurrent removals or administrations → row locks, consistent lock order and fresh stock checks.
- Audit persistence failure → roll back stock and event together; no successful response.
- Historical evidence contains user notes → retain existing household audit controls and omit notes from diagnostics.

## Migration Plan

No schema change. Deploy with the normal Rails release. Rollback removes the entry point while retaining existing stock quantities and audit evidence. Do not reverse valid removals automatically.
