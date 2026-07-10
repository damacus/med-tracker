# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationReviewBenchmarkRunner do
  it 'keeps the committed public-source benchmark free of labelled errors' do
    report = described_class.new.call

    expect(report.fetch('snapshot')).to include('label_count' => 80)
    expect(report.fetch('inventory')).to include('candidate_pair_count' => 3160)
    expect(report.fetch('benchmark')).to include(
      'case_count' => 30, 'false_positives' => 0, 'false_negatives' => 0,
      'precision' => 1.0, 'recall' => 1.0
    )
  end
end
