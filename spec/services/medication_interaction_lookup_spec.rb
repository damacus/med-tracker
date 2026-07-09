# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationInteractionLookup do
  fixtures :all

  it 'maps known combinations to practitioner review prompts' do
    result = described_class.new(medication_scope: Medication.where(id: medications(:aspirin).id)).call(
      dmd_result('Warfarin 1mg tablets')
    )

    expect(result.visible_prompts).to contain_exactly(
      hash_including(expected_review_prompt)
    )
    expect(result.hidden_count).to eq(0)
  end

  it 'returns no review prompts when no review evidence matches' do
    result = described_class.new(medication_scope: Medication.none).call(dmd_result('Cetirizine 10mg tablets'))

    expect(result.visible_prompts).to eq([])
    expect(result.hidden_count).to eq(0)
  end

  it 'filters low-signal prompts while reporting how many were hidden' do
    create_low_signal_evidence

    result = described_class.new(medication_scope: Medication.where(id: medications(:aspirin).id)).call(
      dmd_result('Cetirizine 10mg tablets')
    )

    expect(result.visible_prompts).to eq([])
    expect(result.hidden_count).to eq(1)
  end

  it 'labels unknown risk as unclassified rather than highest risk' do
    expect(described_class.risk_level_label('unknown')).to eq('Unknown - unclassified')
  end

  def dmd_result(display)
    NhsDmd::SearchResult.new(
      code: '3183411000001109',
      display: display,
      system: 'https://dmd.nhs.uk',
      concept_class: 'VMP'
    )
  end

  def expected_review_prompt
    {
      risk_level: 'high', risk_level_label: 'High', match_confidence: 'high', match_confidence_label: 'High',
      interacting_medication_name: 'Aspirin', source_name: 'DailyMed', source_checked_on: '2026-07-09',
      source_url: include('dailymed.nlm.nih.gov'), evidence_text: include('public label'),
      description: include('worth reviewing with a pharmacist, nurse, GP, or prescriber')
    }
  end

  def create_low_signal_evidence
    MedicationReviewEvidenceRecord.create!(
      source_name: 'DailyMed', source_record_id: 'low-signal-test-record',
      source_url: 'https://dailymed.nlm.nih.gov/dailymed/drugInfo.cfm?setid=low-signal-test-record',
      retrieved_on: Date.new(2026, 7, 9), product_name: 'Test medicine', label_section: 'Drug Interactions',
      evidence_text: 'Test-only lower-confidence evidence.', risk_level: 'unknown', match_confidence: 'low',
      match_status: 'reviewed_pair', candidate_terms: %w[cetirizine], interacting_terms: %w[aspirin]
    )
  end
end
