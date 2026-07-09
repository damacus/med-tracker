# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationInteractionLookup do
  fixtures :locations, :medications

  it 'maps known combinations to practitioner review prompts' do
    review_prompts = described_class.new(medication_scope: Medication.where(id: medications(:aspirin).id)).call(
      dmd_result('Warfarin 1mg tablets')
    )

    expect(review_prompts).to contain_exactly(
      hash_including(
        risk_level: 'high',
        risk_level_label: 'High',
        interacting_medication_name: 'Aspirin',
        source_name: 'MedTracker review prompt seed data',
        source_checked_on: Date.current.iso8601,
        description: a_string_including('Review with a pharmacist, nurse, GP, or prescriber')
      )
    )
  end

  it 'returns no review prompts when no review evidence matches' do
    expect(described_class.new(medication_scope: Medication.none).call(dmd_result('Cetirizine 10mg tablets'))).to eq([])
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
end
