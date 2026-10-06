# frozen_string_literal: true

class MedicationReviewEvidenceRefreshRun < ApplicationRecord
  enum :status, { queued: 0, running: 1, completed: 2, failed: 3 }, default: :queued, validate: true

  validates :label_count, :created_count, :updated_count, :unchanged_count, :missing_count,
            numericality: { only_integer: true, greater_than_or_equal_to: 0 }

  def start!
    update!(status: :running, started_at: Time.current)
  end

  def complete!(result)
    update!(
      status: :completed,
      source_last_updated: result.fetch(:source_last_updated),
      label_count: result.fetch(:label_count),
      created_count: result.fetch(:created_count),
      updated_count: result.fetch(:updated_count),
      unchanged_count: result.fetch(:unchanged_count),
      missing_count: result.fetch(:missing_count),
      change_summary: { changes: result.fetch(:changes) },
      completed_at: Time.current,
      error_message: nil
    )
  end

  def fail!(message)
    update!(status: :failed, completed_at: Time.current, error_message: message)
  end
end
