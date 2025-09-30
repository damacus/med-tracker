# frozen_string_literal: true

class AddPersonMedicineToMedicationTakes < ActiveRecord::Migration[8.0]
  def change
    # Make prescription_id nullable to support non-prescription medicines
    change_column_null :medication_takes, :prescription_id, true

    # Add person_medicine_id as nullable reference
    add_reference :medication_takes, :person_medicine, null: true, foreign_key: true

    # Note: SQLite doesn't support ALTER TABLE ADD CONSTRAINT for CHECK constraints
    # We'll enforce the "exactly one source" constraint at the model level instead
  end
end
