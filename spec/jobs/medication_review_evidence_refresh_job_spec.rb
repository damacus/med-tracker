# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationReviewEvidenceRefreshJob do
  let(:service) { instance_double(OpenFda::MedicationReviewEvidenceRefresh) }

  before do
    allow(OpenFda::MedicationReviewEvidenceRefresh).to receive(:new).and_return(service)
  end

  it 'persists a completed refresh report' do
    allow(service).to receive(:call).and_return(
      source_last_updated: Date.new(2026, 7, 10), label_count: 80, created_count: 1,
      updated_count: 2, unchanged_count: 77, missing_count: 0, changes: []
    )

    expect { described_class.perform_now }.to change(MedicationReviewEvidenceRefreshRun, :count).by(1)

    expect(MedicationReviewEvidenceRefreshRun.last).to be_completed
  end

  it 'persists the error and reraises a failed refresh' do
    allow(service).to receive(:call).and_raise(RuntimeError, 'source unavailable')

    expect { described_class.perform_now }.to raise_error(RuntimeError, 'source unavailable')

    expect(MedicationReviewEvidenceRefreshRun.last).to be_failed
    expect(MedicationReviewEvidenceRefreshRun.last.error_message).to eq('source unavailable')
  end

  it 'runs weekly in production' do
    schedule = YAML.safe_load_file(Rails.root.join('config/recurring.yml')).dig(
      'production', 'refresh_medication_review_evidence'
    )

    expect(schedule).to include(
      'class' => described_class.name, 'schedule' => 'every Sunday at 3am', 'queue' => 'default'
    )
  end
end
