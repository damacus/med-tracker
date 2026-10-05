class CreateMedicationPausePeriods < ActiveRecord::Migration[8.1]
  PUBLIC_REASONS = %w[
    out_of_supply
    temporarily_not_needed
    clinician_advice
    side_effects
    other
  ].freeze

  def up
    create_table :medication_pause_periods do |t|
      t.references :household, null: false, foreign_key: true
      t.references :schedule, foreign_key: true
      t.references :person_medication, foreign_key: true
      t.references :recorded_by_membership, foreign_key: { to_table: :household_memberships }
      t.references :resumed_by_membership, foreign_key: { to_table: :household_memberships }
      t.string :portable_id, null: false, default: -> { 'gen_random_uuid()::text' }
      t.string :reason, null: false
      t.text :note
      t.datetime :started_at
      t.datetime :ended_at
      t.boolean :legacy_context, null: false, default: false

      t.timestamps
    end

    add_indexes
    add_check_constraints
    add_household_foreign_keys
    enable_household_rls
  end

  def down
    drop_table :medication_pause_periods
  end

  private

  def add_indexes
    add_index :medication_pause_periods, %i[id household_id], unique: true
    add_index :medication_pause_periods, %i[household_id portable_id], unique: true
    add_index :medication_pause_periods,
              :schedule_id,
              unique: true,
              where: 'ended_at IS NULL AND schedule_id IS NOT NULL',
              name: 'idx_med_pause_periods_open_schedule'
    add_index :medication_pause_periods,
              :person_medication_id,
              unique: true,
              where: 'ended_at IS NULL AND person_medication_id IS NOT NULL',
              name: 'idx_med_pause_periods_open_person_medication'
  end

  def add_check_constraints
    add_check_constraint :medication_pause_periods,
                         'num_nonnulls(schedule_id, person_medication_id) = 1',
                         name: 'chk_medication_pause_periods_exactly_one_source'
    add_check_constraint :medication_pause_periods,
                         'started_at IS NULL OR ended_at IS NULL OR ended_at >= started_at',
                         name: 'chk_medication_pause_periods_interval'
    add_check_constraint :medication_pause_periods,
                         reason_constraint,
                         name: 'chk_medication_pause_periods_reason'
    add_check_constraint :medication_pause_periods,
                         legacy_context_constraint,
                         name: 'chk_medication_pause_periods_legacy_context'
    add_check_constraint :medication_pause_periods,
                         resuming_actor_constraint,
                         name: 'chk_medication_pause_periods_resuming_actor'
  end

  def reason_constraint
    reasons = (PUBLIC_REASONS + ['reason_not_recorded']).map { |reason| quote(reason) }.join(', ')
    "reason IN (#{reasons})"
  end

  def legacy_context_constraint
    <<~SQL.squish
      (legacy_context = TRUE AND reason = 'reason_not_recorded') OR
      (legacy_context = FALSE AND reason <> 'reason_not_recorded' AND started_at IS NOT NULL AND
       recorded_by_membership_id IS NOT NULL)
    SQL
  end

  def resuming_actor_constraint
    <<~SQL.squish
      (ended_at IS NULL AND resumed_by_membership_id IS NULL) OR
      (ended_at IS NOT NULL AND resumed_by_membership_id IS NOT NULL)
    SQL
  end

  def add_household_foreign_keys
    add_household_foreign_key(:schedule_id, :schedules, 'fk_med_pause_periods_schedule_household')
    add_household_foreign_key(
      :person_medication_id,
      :person_medications,
      'fk_med_pause_periods_person_medication_household'
    )
    add_household_foreign_key(
      :recorded_by_membership_id,
      :household_memberships,
      'fk_med_pause_periods_recorded_actor_household'
    )
    add_household_foreign_key(
      :resumed_by_membership_id,
      :household_memberships,
      'fk_med_pause_periods_resumed_actor_household'
    )
  end

  def add_household_foreign_key(column, target_table, name)
    add_foreign_key :medication_pause_periods,
                    target_table,
                    column: [column, :household_id],
                    primary_key: %i[id household_id],
                    validate: false,
                    name: name
  end

  def enable_household_rls
    execute 'ALTER TABLE medication_pause_periods ENABLE ROW LEVEL SECURITY;'
    execute 'ALTER TABLE medication_pause_periods FORCE ROW LEVEL SECURITY;'
    execute <<~SQL.squish
      CREATE POLICY household_tenant_isolation ON medication_pause_periods
      USING (household_id = med_tracker.current_household_id())
      WITH CHECK (household_id = med_tracker.current_household_id());
    SQL
  end
end
