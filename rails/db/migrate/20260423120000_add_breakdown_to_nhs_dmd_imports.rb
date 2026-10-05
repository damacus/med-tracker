# frozen_string_literal: true

class AddBreakdownToNhsDmdImports < ActiveRecord::Migration[8.0]
  def change
    change_table :nhs_dmd_imports, bulk: true do |t|
      t.integer :created_count, null: false, default: 0
      t.integer :updated_count, null: false, default: 0
      t.integer :unchanged_count, null: false, default: 0
      t.integer :skipped_expired_count, null: false, default: 0
      t.integer :skipped_missing_name_count, null: false, default: 0
      t.integer :skipped_invalid_count, null: false, default: 0
    end
  end
end
