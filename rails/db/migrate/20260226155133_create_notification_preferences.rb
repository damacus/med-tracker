class CreateNotificationPreferences < ActiveRecord::Migration[8.1]
  def change
    create_table :notification_preferences do |t|
      t.references :person, null: false, foreign_key: { deferrable: :deferred }, index: { unique: true }
      t.boolean :enabled, default: true, null: false
      t.time :morning_time, default: "08:00"
      t.time :afternoon_time, default: "14:00"
      t.time :evening_time, default: "18:00"
      t.time :night_time, default: "22:00"

      t.timestamps
    end
  end
end
