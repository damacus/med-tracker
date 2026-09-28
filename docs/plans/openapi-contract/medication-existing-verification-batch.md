# Existing medication API verification

Verify listMedications, createMedication, getMedication, updateMedication,
replaceMedication, listMedicationTakes and createMedicationTake after the
assignment and schedule write batches. These seven routes already exist;
route presence alone receives no completion credit.

Existing Rust runtime targets medication_read_api, medication_stock and
dose_write_api exercise the successful operations and substantial access,
stock, dose and concurrency behaviour. The medication-read-api runner mode
uses the Rust API even though its first argument is rails.

Luna owns a new openapi_medications.rs target for missing strict request and
response assertions and endpoint-specific boundary/rate wiring. Reuse the
existing targets as behavioural proof rather than duplicating their tests.
Map each operation's documented statuses and required/allowed fields to these
four targets. Inspect fixture assumptions and actual OpenAPI schemas before
asserting response shapes. Correct known defects instead of weakening tests.

Sol retains runner and production ownership. Compile new tests, run the
selected combined targets and capture any actual RED before corrections.
Independent review plus passing combined isolated acceptance, applicable
Rust/runner gates and publication are required before counting the seven
operations complete. Preserve the original 89-operation baseline separately;
these already-present operations can improve the 118 count without changing
the original baseline count.
