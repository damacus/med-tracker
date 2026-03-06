# frozen_string_literal: true

class AddTakenFromSourceToMedicationTakes < ActiveRecord::Migration[8.1]
  def change
    add_reference :medication_takes, :taken_from_medication, foreign_key: { to_table: :medications }
    add_reference :medication_takes, :taken_from_location, foreign_key: { to_table: :locations }
  end
end
