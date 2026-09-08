class CreateMedicationDoseOccurrences < ActiveRecord::Migration[8.1]
  def up
    create_table :medication_dose_occurrences do |t|
      t.references :household, null: false, foreign_key: true
      t.references :schedule, foreign_key: true
      t.references :person_medication, foreign_key: true
      t.references :medication_take, foreign_key: true, index: { unique: true }
      t.references :resolved_by_membership, foreign_key: { to_table: :household_memberships }
      t.string :portable_id, null: false, default: -> { 'gen_random_uuid()::text' }
      t.date :window_starts_on, null: false
      t.integer :position, null: false
      t.datetime :scheduled_at
      t.string :outcome, null: false, default: 'open'
      t.string :reason
      t.text :note
      t.datetime :resolved_at
      t.timestamps
    end

    add_identity_indexes
    add_state_constraints
    add_household_foreign_keys
    enable_household_rls
  end

  def down
    drop_table :medication_dose_occurrences
  end

  private

  def add_identity_indexes
    add_index :medication_dose_occurrences, %i[id household_id], unique: true
    add_index :medication_dose_occurrences, %i[household_id portable_id], unique: true,
              name: 'idx_dose_occurrences_household_portable_id'
    %i[schedule_id person_medication_id].each do |source|
      add_index :medication_dose_occurrences, [source, :window_starts_on, :position],
                unique: true, where: "#{source} IS NOT NULL", name: "idx_dose_occurrence_#{source}_window"
    end
  end

  def add_state_constraints
    add_check_constraint :medication_dose_occurrences, 'num_nonnulls(schedule_id, person_medication_id) = 1',
                         name: 'chk_dose_occurrences_exact_source'
    add_check_constraint :medication_dose_occurrences, 'position > 0', name: 'chk_dose_occurrences_position'
    add_check_constraint :medication_dose_occurrences, state_constraint, name: 'chk_dose_occurrences_state'
    add_check_constraint :medication_dose_occurrences,
                         "reason IS NULL OR reason IN ('refused', 'unwell', 'asleep', 'medicine_unavailable', 'clinician_advice', 'other')",
                         name: 'chk_dose_occurrences_reason'
    add_check_constraint :medication_dose_occurrences, 'note IS NULL OR char_length(note) <= 2000',
                         name: 'chk_dose_occurrences_note'
  end

  def state_constraint
    <<~SQL.squish
      (outcome = 'open' AND medication_take_id IS NULL AND reason IS NULL AND note IS NULL
        AND resolved_at IS NULL AND resolved_by_membership_id IS NULL) OR
      (outcome = 'not_taken' AND medication_take_id IS NULL
        AND resolved_at IS NOT NULL AND resolved_by_membership_id IS NOT NULL) OR
      (outcome = 'taken' AND medication_take_id IS NOT NULL AND reason IS NULL AND note IS NULL
        AND resolved_at IS NOT NULL AND resolved_by_membership_id IS NOT NULL)
    SQL
  end

  def add_household_foreign_keys
    { schedule_id: :schedules, person_medication_id: :person_medications,
      medication_take_id: :medication_takes, resolved_by_membership_id: :household_memberships }.each do |column, target|
      add_foreign_key :medication_dose_occurrences, target, column: [column, :household_id],
                      primary_key: %i[id household_id], name: "fk_dose_occurrence_#{column}_household"
    end
  end

  def enable_household_rls
    execute 'ALTER TABLE medication_dose_occurrences ENABLE ROW LEVEL SECURITY'
    execute 'ALTER TABLE medication_dose_occurrences FORCE ROW LEVEL SECURITY'
    execute <<~SQL.squish
      CREATE POLICY household_tenant_isolation ON medication_dose_occurrences
      USING (household_id = med_tracker.current_household_id())
      WITH CHECK (household_id = med_tracker.current_household_id())
    SQL
  end
end
