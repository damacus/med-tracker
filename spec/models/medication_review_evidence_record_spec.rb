# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationReviewEvidenceRecord do
  subject(:record) { described_class.new(valid_attributes) }

  let(:valid_attributes) do
    {
      source_name: 'DailyMed',
      source_record_id: 'evidence-record-model-spec',
      source_version: '4',
      source_effective_on: Date.new(2026, 7, 1),
      source_url: 'https://dailymed.nlm.nih.gov/dailymed/drugInfo.cfm?setid=evidence-record-model-spec',
      retrieved_on: Date.new(2026, 7, 9),
      product_name: 'Warfarin Sodium',
      active_ingredient: 'Warfarin sodium',
      label_section: 'Drug Interactions',
      evidence_text: 'The label identifies increased bleeding risk with some medicines.',
      risk_level: 'high',
      match_confidence: 'high',
      match_status: 'reviewed_pair',
      candidate_terms: %w[warfarin],
      interacting_terms: %w[ibuprofen]
    }
  end

  it 'accepts a reviewed public-label medicine pair' do
    expect(record).to be_valid
  end

  it 'rejects unsupported risk levels' do
    record.risk_level = 'critical'

    expect(record).not_to be_valid
    expect(record.errors[:risk_level]).to be_present
  end

  it 'requires reviewed pairs to contain terms for both medicines' do
    record.interacting_terms = []

    expect(record).not_to be_valid
    expect(record.errors[:interacting_terms]).to be_present
  end

  it 'matches reviewed pairs in either medicine order' do
    expect(record.match_pair?(candidate_name: 'Warfarin 1mg tablets',
                              existing_name: 'Ibuprofen 200mg tablets')).to be(true)
    expect(record.match_pair?(candidate_name: 'Ibuprofen', existing_name: 'Warfarin sodium')).to be(true)
  end

  it 'does not treat an unreviewed label as a manually curated pair' do
    record.match_status = 'unreviewed'

    expect(record.match_pair?(candidate_name: 'Warfarin', existing_name: 'Ibuprofen')).to be(false)
  end
end
