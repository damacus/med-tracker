# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationReviewBenchmark do
  let(:evidence) do
    MedicationReviewEvidenceRecord.new(
      source_name: 'DailyMed', source_record_id: 'benchmark-evidence', source_version: '1',
      source_effective_on: Date.new(2026, 7, 1), source_url: 'https://dailymed.nlm.nih.gov/example',
      retrieved_on: Date.new(2026, 7, 10), product_name: 'Warfarin', active_ingredient: 'Warfarin',
      label_section: 'Drug Interactions', evidence_text: 'Monitor patients receiving ibuprofen for bleeding.',
      risk_level: 'unknown', match_confidence: 'unknown', match_status: 'unreviewed',
      candidate_terms: ['warfarin'], pharmacologic_classes: [], interacting_terms: []
    )
  end
  let(:cases) do
    [
      { 'id' => 'true-positive', 'first' => 'warfarin', 'second' => 'ibuprofen', 'expected_match' => true },
      { 'id' => 'true-negative', 'first' => 'warfarin', 'second' => 'metformin', 'expected_match' => false },
      { 'id' => 'false-positive', 'first' => 'warfarin', 'second' => 'ibuprofen', 'expected_match' => false },
      { 'id' => 'false-negative', 'first' => 'warfarin', 'second' => 'metformin', 'expected_match' => true }
    ]
  end
  let(:inventory_expectation) do
    {
      'candidate_pair_count' => 3, 'matched_pair_count' => 1,
      'match_count' => 1, 'match_types' => { 'ingredient' => 1 }
    }
  end
  let(:benchmark_expectation) do
    {
      'case_count' => 4, 'true_positives' => 1, 'true_negatives' => 1,
      'false_positives' => 1, 'false_negatives' => 1, 'precision' => 0.5, 'recall' => 0.5
    }
  end

  it 'measures the full selected-pair inventory and finite labelled cases' do
    report = described_class.new(records: [evidence], selection: %w[warfarin ibuprofen metformin], cases: cases).call

    expect(report.fetch('inventory')).to include(inventory_expectation)
    expect(report.fetch('benchmark')).to include(benchmark_expectation)
  end
end
