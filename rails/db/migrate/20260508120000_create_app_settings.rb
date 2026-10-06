# frozen_string_literal: true

class CreateAppSettings < ActiveRecord::Migration[8.1]
  def change
    create_table :app_settings do |t|
      t.boolean :invite_only, null: false, default: false
      t.timestamps
    end
  end
end
