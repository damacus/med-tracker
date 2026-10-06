# frozen_string_literal: true

class AddMatchExplanationToMedicationReviewPrompts < ActiveRecord::Migration[8.1]
  def up
    add_column :medication_review_prompts, :evidence_source_version, :string
    add_column :medication_review_prompts, :evidence_source_effective_on, :date
    add_column :medication_review_prompts, :matched_term, :string
    add_column :medication_review_prompts, :match_type, :string
    add_column :medication_review_prompts, :source_instruction, :string
    add_column :medication_review_prompts, :match_reason, :text

    execute <<~SQL.squish
      UPDATE medication_review_prompts
      SET evidence_source_version = 'legacy',
          evidence_source_effective_on = evidence_source_checked_on,
          matched_term = interacting_medication_name,
          match_type = 'legacy',
          source_instruction = 'unclassified',
          match_reason = 'Created before automatic match explanations were recorded.'
    SQL

    change_column_null :medication_review_prompts, :evidence_source_version, false
    change_column_null :medication_review_prompts, :evidence_source_effective_on, false
    change_column_null :medication_review_prompts, :matched_term, false
    change_column_null :medication_review_prompts, :match_type, false
    change_column_null :medication_review_prompts, :source_instruction, false
    change_column_null :medication_review_prompts, :match_reason, false
  end

  def down
    remove_column :medication_review_prompts, :match_reason
    remove_column :medication_review_prompts, :source_instruction
    remove_column :medication_review_prompts, :match_type
    remove_column :medication_review_prompts, :matched_term
    remove_column :medication_review_prompts, :evidence_source_effective_on
    remove_column :medication_review_prompts, :evidence_source_version
  end
end
