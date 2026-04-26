# frozen_string_literal: true

class CreateNhsDmdImports < ActiveRecord::Migration[8.0]
  def change
    create_table :nhs_dmd_imports do |t|
      t.string :uploaded_filename, null: false
      t.string :archive_path
      t.integer :status, null: false, default: 0
      t.integer :total_records, null: false, default: 0
      t.integer :processed_records, null: false, default: 0
      t.integer :imported_count, null: false, default: 0
      t.integer :skipped_count, null: false, default: 0
      t.text :log
      t.text :error_message
      t.datetime :started_at
      t.datetime :completed_at

      t.timestamps
    end
  end
end
