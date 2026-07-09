# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationReviewPromptSync do
  fixtures :all

  let(:household) { households(:fixture_household) }
  let(:person) { people(:john) }
  let(:warfarin) do
    household.medications.create!(
      name: 'Warfarin 1mg tablets',
      location: locations(:home),
      dose_amount: 1,
      dose_unit: 'tablet',
      current_supply: 28,
      reorder_threshold: 7
    )
  end

  before do
    PersonMedication.create!(
      household: household,
      person: person,
      medication: warfarin,
      dose_amount: 1,
      dose_unit: 'tablet',
      administration_kind: 'routine'
    )
    PersonMedication.create!(
      household: household,
      person: person,
      medication: medications(:ibuprofen),
      dose_amount: 200,
      dose_unit: 'mg',
      administration_kind: 'as_needed'
    )
  end

  it 'creates a prompt with a stable evidence snapshot for an active medicine pair' do
    expect do
      described_class.new(people: Person.where(id: person.id)).call
    end.to change(MedicationReviewPrompt, :count).by(1)

    prompt = MedicationReviewPrompt.last
    expect(prompt).to have_attributes(
      household: household,
      person: person,
      primary_medication: warfarin,
      interacting_medication: medications(:ibuprofen),
      status: 'needs_review',
      risk_level: 'high',
      match_confidence: 'high',
      evidence_source_name: 'DailyMed',
      evidence_source_checked_on: Date.new(2026, 7, 9)
    )
  end

  it 'does not duplicate or reset an existing prompt' do
    sync = described_class.new(people: Person.where(id: person.id))
    sync.call
    prompt = MedicationReviewPrompt.last
    prompt.update!(status: 'not_relevant')

    expect { sync.call }.not_to change(MedicationReviewPrompt, :count)
    expect(prompt.reload.status).to eq('not_relevant')
  end

  it 'marks low-confidence evidence as hidden low signal' do
    create_low_confidence_evidence

    described_class.new(people: Person.where(id: person.id)).call

    expect(MedicationReviewPrompt.hidden_low_signal.count).to eq(1)
  end

  def create_low_confidence_evidence
    MedicationReviewEvidenceRecord.create!(
      source_name: 'DailyMed', source_record_id: 'low-confidence-sync-spec',
      source_url: 'https://dailymed.nlm.nih.gov/dailymed/drugInfo.cfm?setid=low-confidence-sync-spec',
      retrieved_on: Date.new(2026, 7, 9), product_name: 'Warfarin Sodium', label_section: 'Drug Interactions',
      evidence_text: 'Test-only low-confidence pairing.', risk_level: 'unknown', match_confidence: 'low',
      match_status: 'reviewed_pair', candidate_terms: %w[warfarin], interacting_terms: %w[ibuprofen]
    )
  end
end
