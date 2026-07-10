# frozen_string_literal: true

class AddSourceMetadataToMedicationReviewEvidence < ActiveRecord::Migration[8.1]
  def change
    add_column :medication_review_evidence_records, :source_version, :string
    add_column :medication_review_evidence_records, :source_effective_on, :date
  end
end
