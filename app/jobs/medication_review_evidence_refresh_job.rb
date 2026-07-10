# frozen_string_literal: true

class MedicationReviewEvidenceRefreshJob < ApplicationJob
  queue_as :default

  def perform
    run = MedicationReviewEvidenceRefreshRun.create!
    run.start!
    run.complete!(OpenFda::MedicationReviewEvidenceRefresh.new.call)
    run
  rescue StandardError => e
    run&.fail!(e.message)
    raise
  end
end
