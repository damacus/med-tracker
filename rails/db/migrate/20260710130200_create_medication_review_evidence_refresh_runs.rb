# frozen_string_literal: true

class CreateMedicationReviewEvidenceRefreshRuns < ActiveRecord::Migration[8.1]
  def change
    create_table :medication_review_evidence_refresh_runs do |t|
      t.integer :status, default: 0, null: false
      t.date :source_last_updated
      t.datetime :started_at
      t.datetime :completed_at
      t.integer :label_count, default: 0, null: false
      t.integer :created_count, default: 0, null: false
      t.integer :updated_count, default: 0, null: false
      t.integer :unchanged_count, default: 0, null: false
      t.integer :missing_count, default: 0, null: false
      t.jsonb :change_summary, default: {}, null: false
      t.text :error_message
      t.timestamps
    end
  end
end
