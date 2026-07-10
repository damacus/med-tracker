# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationReviewEvidenceRefreshRun do
  it 'records a completed source refresh summary' do
    run = described_class.create!
    result = {
      source_last_updated: Date.new(2026, 7, 10), label_count: 80, created_count: 2,
      updated_count: 3, unchanged_count: 74, missing_count: 1,
      changes: [{ type: 'updated', source_record_id: 'label-id' }]
    }

    run.start!
    run.complete!(result)

    expect(run).to be_completed
    expect(run).to have_attributes(result.except(:changes).merge(error_message: nil))
    expect(run.change_summary).to eq('changes' => [{ 'type' => 'updated', 'source_record_id' => 'label-id' }])
    expect(run.started_at).to be_present
    expect(run.completed_at).to be_present
  end

  it 'retains a refresh failure for investigation' do
    run = described_class.create!

    run.start!
    run.fail!('openFDA unavailable')

    expect(run).to be_failed
    expect(run).to have_attributes(error_message: 'openFDA unavailable')
    expect(run.completed_at).to be_present
  end
end
