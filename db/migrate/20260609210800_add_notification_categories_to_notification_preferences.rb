# frozen_string_literal: true

class AddNotificationCategoriesToNotificationPreferences < ActiveRecord::Migration[8.1]
  def change
    add_column :notification_preferences, :dose_due_enabled, :boolean, default: true, null: false
    add_column :notification_preferences, :missed_dose_enabled, :boolean, default: true, null: false
    add_column :notification_preferences, :low_stock_enabled, :boolean, default: true, null: false
    add_column :notification_preferences, :private_text_enabled, :boolean, default: true, null: false
  end
end
