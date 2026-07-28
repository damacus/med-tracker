# frozen_string_literal: true

class AddMissedDoseNotificationsToPersonAccessGrants < ActiveRecord::Migration[8.1]
  def change
    add_column :person_access_grants, :missed_dose_notifications_enabled, :boolean, default: false, null: false
  end
end
