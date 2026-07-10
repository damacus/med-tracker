# frozen_string_literal: true

class AddPharmacologicClassesToMedicationReviewEvidenceRecords < ActiveRecord::Migration[8.1]
  def change
    add_column :medication_review_evidence_records, :pharmacologic_classes, :string, array: true, null: false,
                                                                                         default: []
    add_index :medication_review_evidence_records, :pharmacologic_classes, using: :gin
  end
end
