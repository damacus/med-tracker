# frozen_string_literal: true

class BackfillLegacyMedicationPausePeriods < ActiveRecord::Migration[8.1]
  SOURCES = {
    schedules: :schedule_id,
    person_medications: :person_medication_id
  }.freeze

  def up
    return unless required_tables?

    connection.transaction do
      household_ids.each do |household_id|
        set_household_context(household_id)
        SOURCES.each { |table_name, source_column| backfill_sources(table_name, source_column, household_id) }
      end
    end
  end

  def down; end

  private

  def required_tables?
    %i[households schedules person_medications medication_pause_periods].all? do |table_name|
      table_exists?(table_name)
    end
  end

  def household_ids
    connection.select_values('SELECT id FROM households ORDER BY id')
  end

  def set_household_context(household_id)
    execute <<~SQL.squish
      SELECT set_config('med_tracker.current_household_id', #{quote(household_id.to_s)}, true)
    SQL
  end

  def backfill_sources(table_name, source_column, household_id)
    candidate_ids(table_name, source_column, household_id).each do |source_id|
      lock_source(table_name, household_id, source_id)
      insert_period(table_name, source_column, household_id, source_id)
    end
  end

  def candidate_ids(table_name, source_column, household_id)
    connection.select_values <<~SQL.squish
      SELECT sources.id
      FROM #{table_name} sources
      WHERE sources.household_id = #{quote(household_id)}
        AND sources.active = FALSE
        AND sources.retired_at IS NULL
        AND NOT EXISTS (
          SELECT 1
          FROM medication_pause_periods
          WHERE medication_pause_periods.#{source_column} = sources.id
            AND medication_pause_periods.ended_at IS NULL
        )
      ORDER BY sources.id
    SQL
  end

  def lock_source(table_name, household_id, source_id)
    execute <<~SQL.squish
      SELECT id
      FROM #{table_name}
      WHERE household_id = #{quote(household_id)}
        AND id = #{quote(source_id)}
      FOR UPDATE
    SQL
  end

  def insert_period(table_name, source_column, household_id, source_id)
    execute <<~SQL.squish
      INSERT INTO medication_pause_periods (
        household_id, #{source_column}, reason, started_at, ended_at, legacy_context, created_at, updated_at
      )
      SELECT sources.household_id,
             sources.id,
             'reason_not_recorded',
             NULL,
             NULL,
             TRUE,
             CURRENT_TIMESTAMP,
             CURRENT_TIMESTAMP
      FROM #{table_name} sources
      WHERE sources.household_id = #{quote(household_id)}
        AND sources.id = #{quote(source_id)}
        AND sources.active = FALSE
        AND sources.retired_at IS NULL
        AND NOT EXISTS (
          SELECT 1
          FROM medication_pause_periods
          WHERE medication_pause_periods.#{source_column} = sources.id
            AND medication_pause_periods.ended_at IS NULL
        )
      ON CONFLICT (#{source_column})
        WHERE ended_at IS NULL AND #{source_column} IS NOT NULL DO NOTHING
    SQL
  end
end
