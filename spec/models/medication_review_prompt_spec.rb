# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationReviewPrompt do
  fixtures :all

  subject(:prompt) do
    described_class.new(
      household: households(:fixture_household),
      person: people(:john),
      primary_medication: medications(:aspirin),
      interacting_medication: medications(:ibuprofen),
      evidence_record: evidence_record,
      status: 'needs_review',
      risk_level: 'high',
      match_confidence: 'high',
      primary_medication_name: 'Aspirin',
      interacting_medication_name: 'Ibuprofen',
      evidence_source_name: 'DailyMed',
      evidence_source_url: evidence_record.source_url,
      evidence_source_checked_on: Date.new(2026, 7, 9),
      evidence_text: evidence_record.evidence_text
    )
  end

  let(:evidence_record) do
    MedicationReviewEvidenceRecord.create!(
      source_name: 'DailyMed',
      source_record_id: 'warfarin-nsaids-model-spec',
      source_url: 'https://dailymed.nlm.nih.gov/dailymed/drugInfo.cfm?setid=test',
      retrieved_on: Date.new(2026, 7, 9),
      product_name: 'Warfarin Sodium',
      active_ingredient: 'Warfarin sodium',
      label_section: 'Drug Interactions',
      evidence_text: 'Public label evidence for practitioner review.',
      risk_level: 'high',
      match_confidence: 'high',
      match_status: 'reviewed_pair',
      candidate_terms: %w[aspirin],
      interacting_terms: %w[ibuprofen]
    )
  end

  it 'accepts a household-owned prompt with an evidence snapshot' do
    expect(prompt).to be_valid
  end

  it 'requires practitioner details when recorded as reviewed' do
    prompt.status = 'reviewed_with_practitioner'

    expect(prompt).not_to be_valid
    expect(prompt.errors[:practitioner_name]).to be_present
    expect(prompt.errors[:practitioner_role]).to be_present
    expect(prompt.errors[:reviewed_on]).to be_present
  end

  it 'accepts an expected prescribed combination with practitioner details' do
    prompt.assign_attributes(
      status: 'expected_prescribed_combination',
      practitioner_name: 'Dr Taylor',
      practitioner_role: 'GP',
      reviewed_on: Date.new(2026, 7, 9)
    )

    expect(prompt).to be_valid
  end

  it 'does not allow the saved evidence snapshot to be rewritten' do
    prompt.save!
    prompt.evidence_text = 'Replacement evidence'

    expect(prompt).not_to be_valid
    expect(prompt.errors[:evidence_text]).to include('cannot be changed after the review prompt is created')
  end

  it 'does not allow the saved medication pair to be replaced' do
    prompt.save!
    prompt.interacting_medication = medications(:paracetamol)

    expect(prompt).not_to be_valid
    expect(prompt.errors[:interacting_medication_id])
      .to include('cannot be changed after the review prompt is created')
  end

  it 'rejects medications from another household' do
    foreign_household = Household.create!(name: 'Other household', timezone: Time.zone.name)
    foreign_location = foreign_household.locations.create!(name: 'Other location')
    prompt.interacting_medication = foreign_household.medications.create!(
      name: 'Other medicine',
      location: foreign_location,
      dose_amount: 1,
      dose_unit: 'tablet',
      current_supply: 1,
      reorder_threshold: 1
    )

    expect(prompt).not_to be_valid
    expect(prompt.errors[:interacting_medication]).to be_present
  end
end
