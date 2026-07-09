# frozen_string_literal: true

class CreateMedicationReviewEvidenceRecords < ActiveRecord::Migration[8.1]
  def change
    create_table :medication_review_evidence_records do |t|
      t.string :source_name, null: false
      t.string :source_record_id, null: false
      t.string :source_url, null: false
      t.date :retrieved_on, null: false
      t.string :product_name, null: false
      t.string :active_ingredient
      t.string :label_section, null: false
      t.text :evidence_text, null: false
      t.string :risk_level, null: false, default: 'unknown'
      t.string :match_confidence, null: false, default: 'unknown'
      t.string :match_status, null: false, default: 'unreviewed'
      t.string :candidate_terms, array: true, null: false, default: []
      t.string :interacting_terms, array: true, null: false, default: []

      t.timestamps
    end

    add_index :medication_review_evidence_records, :source_record_id, unique: true
    add_index :medication_review_evidence_records, :match_status
    add_index :medication_review_evidence_records, :candidate_terms, using: :gin
    add_index :medication_review_evidence_records, :interacting_terms, using: :gin
  end
end
