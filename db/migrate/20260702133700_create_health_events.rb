# frozen_string_literal: true

class CreateHealthEvents < ActiveRecord::Migration[8.1]
  def up
    create_table :health_events do |t|
      t.references :household, null: false, foreign_key: true
      t.references :person, null: false, foreign_key: true
      t.integer :event_kind, null: false
      t.string :title, null: false
      t.date :started_on, null: false
      t.date :ended_on
      t.integer :severity
      t.text :notes
      t.text :action_taken
      t.boolean :medical_help_sought, null: false, default: false

      t.timestamps
    end

    add_index :health_events, %i[person_id started_on ended_on]
    add_index :health_events, %i[person_id event_kind started_on]
    add_index :health_events, %i[id household_id], unique: true
    add_foreign_key :health_events,
                    :people,
                    column: %i[person_id household_id],
                    primary_key: %i[id household_id],
                    validate: false,
                    name: 'fk_health_events_person_id_household'

    create_table :health_event_medications do |t|
      t.references :household, null: false, foreign_key: true
      t.references :health_event, null: false, foreign_key: true
      t.references :medication, foreign_key: true
      t.string :medication_name, null: false

      t.timestamps
    end

    add_index :health_event_medications, %i[health_event_id medication_id],
              unique: true,
              where: 'medication_id IS NOT NULL',
              name: 'index_health_event_medications_on_health_event_id_and_med_id'
    add_index :health_event_medications, %i[id household_id], unique: true
    add_foreign_key :health_event_medications,
                    :health_events,
                    column: %i[health_event_id household_id],
                    primary_key: %i[id household_id],
                    validate: false,
                    name: 'fk_health_event_medications_health_event_id_household'
    add_foreign_key :health_event_medications,
                    :medications,
                    column: %i[medication_id household_id],
                    primary_key: %i[id household_id],
                    validate: false,
                    name: 'fk_health_event_medications_medication_id_household'

    enable_household_rls(:health_events)
    enable_household_rls(:health_event_medications)
  end

  def down
    drop_table :health_event_medications
    drop_table :health_events
  end

  private

  def enable_household_rls(table_name)
    quoted_table = quote_table_name(table_name)

    execute "ALTER TABLE #{quoted_table} ENABLE ROW LEVEL SECURITY;"
    execute "ALTER TABLE #{quoted_table} FORCE ROW LEVEL SECURITY;"
    execute <<~SQL.squish
      CREATE POLICY household_tenant_isolation ON #{quoted_table}
      USING (household_id = med_tracker.current_household_id())
      WITH CHECK (household_id = med_tracker.current_household_id());
    SQL
  end
end
