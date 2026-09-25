# Resolve dosage policy and domain gaps

Read-only findings after the dosage checkpoint, to apply after the active
session-management tranche. The global default is now to document existing
Rails rules before deriving OpenAPI tests, preserving the original parity goal.

- `MedicationDosageOptionPolicy#index?` allows active household members.
  `DosagePolicy#show?` delegates to `MedicationPolicy#show?`. Owners and
  administrators see household medications; other members require the existing
  person-grant or delegated-created-unlinked medication visibility rule.
- Creation and update delegate to medication update permission: household
  owner/administrator and same household. Replace the provisional owner-only
  guard and its explicitly temporary administrator-denial assertion.
- The Rails dosage scope currently selects all household dosage records, while
  detail authorization applies medication visibility. This is a potential list
  disclosure of records whose detail is denied. Add an explicit hidden-medication
  regression and use consistent medication visibility; do not reproduce this
  discrepancy as required compatibility.
- `MedicationDosageOption` validates amount greater than zero, minimum dose
  interval at least zero, optional inventory values at least zero, and nonblank
  unit/frequency. Document these rules and test individual validation failures.
- The model has create/inventory synchronization callbacks. Inspect their exact
  observable effects before claiming the dosage operations complete. Default
  option uniqueness, switching and rollback need verified behaviour.
  Creation clears the medication's single-dose amount and refreshes its updated
  timestamp/sync version. A supplied or changed option stock value recalculates
  medication stock from tracked options, sums their reorder thresholds, and
  updates the last-restock baseline. See
  `Medication#sync_inventory_from_dosage_records!` and its reset/restock helpers.
  Keep these related changes atomic and use a consistent parent-before-child
  lock order with location deletion.
- Validated new records require the non-null fields in OpenAPI. Nullable legacy
  database records need an explicit compatibility/data-quality resolution;
  neither invented dosage values nor silently hiding records is acceptable.

Sources: `app/policies/medication_dosage_option_policy.rb`,
`app/policies/dosage_policy.rb`, `app/policies/medication_policy.rb`,
`app/controllers/api/v1/dosage_options_controller.rb`,
`app/models/medication_dosage_option.rb`, and
`app/serializers/api/v1/dosage_option_serializer.rb`.
