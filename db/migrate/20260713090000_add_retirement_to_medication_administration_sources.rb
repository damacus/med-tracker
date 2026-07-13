# frozen_string_literal: true

class AddRetirementToMedicationAdministrationSources < ActiveRecord::Migration[8.1]
  PERSON_MEDICATION_UNIQUE_INDEX = 'index_person_medications_on_person_id_and_medication_id'

  def up
    add_column :schedules, :retired_at, :datetime
    add_column :person_medications, :retired_at, :datetime

    add_index :schedules, :retired_at
    add_index :person_medications, :retired_at
    remove_index :person_medications, name: PERSON_MEDICATION_UNIQUE_INDEX
    add_index :person_medications, %i[person_id medication_id], unique: true, where: 'retired_at IS NULL',
                                                                 name: PERSON_MEDICATION_UNIQUE_INDEX
  end

  def down
    remove_index :person_medications, name: PERSON_MEDICATION_UNIQUE_INDEX
    add_index :person_medications, %i[person_id medication_id], unique: true,
                                                                 name: PERSON_MEDICATION_UNIQUE_INDEX
    remove_index :person_medications, :retired_at
    remove_index :schedules, :retired_at
    remove_column :person_medications, :retired_at
    remove_column :schedules, :retired_at
  end
end
