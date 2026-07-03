# frozen_string_literal: true

class CreateNotificationEvents < ActiveRecord::Migration[8.1]
  def up
    create_table :notification_events do |t|
      t.references :household, null: false, foreign_key: { deferrable: :deferred }
      t.references :person, null: true, foreign_key: { deferrable: :deferred }
      t.string :event_type, null: false
      t.string :event_key, null: false
      t.jsonb :metadata, default: {}, null: false
      t.datetime :sent_at
      t.string :skipped_reason

      t.timestamps

      t.index %i[event_type event_key], unique: true
      t.index %i[id household_id], unique: true
      t.index :sent_at
    end

    add_foreign_key :notification_events,
                    :people,
                    column: %i[person_id household_id],
                    primary_key: %i[id household_id],
                    validate: false,
                    name: 'fk_notification_events_person_id_household'

    execute 'ALTER TABLE notification_events ENABLE ROW LEVEL SECURITY;'
    execute 'ALTER TABLE notification_events FORCE ROW LEVEL SECURITY;'
    execute <<~SQL.squish
      CREATE POLICY household_tenant_isolation ON notification_events
      USING (household_id = med_tracker.current_household_id())
      WITH CHECK (household_id = med_tracker.current_household_id());
    SQL
  end

  def down
    drop_table :notification_events
  end
end
