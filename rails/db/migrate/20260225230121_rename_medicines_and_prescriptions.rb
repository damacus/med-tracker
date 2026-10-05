class RenameMedicinesAndPrescriptions < ActiveRecord::Migration[8.1]
  def change
    # 1. Rename 'medicines' -> 'medications'
    rename_table :medicines, :medications

    # 2. Rename 'prescriptions' -> 'schedules'
    rename_table :prescriptions, :schedules

    # 3. Rename 'person_medicines' -> 'person_medications'
    rename_table :person_medicines, :person_medications

    # 4. Rename foreign keys across tables
    
    # In dosages: medicine_id -> medication_id
    rename_column :dosages, :medicine_id, :medication_id

    # In schedules (formerly prescriptions): medicine_id -> medication_id
    rename_column :schedules, :medicine_id, :medication_id

    # In person_medications (formerly person_medicines): medicine_id -> medication_id
    rename_column :person_medications, :medicine_id, :medication_id

    # In medication_takes
    rename_column :medication_takes, :prescription_id, :schedule_id
    rename_column :medication_takes, :person_medicine_id, :person_medication_id

    # Note: Rails 8 automatically renames index and foreign key constraints when using rename_table/rename_column 
    # if they were created with standard conventions.
  end
end
