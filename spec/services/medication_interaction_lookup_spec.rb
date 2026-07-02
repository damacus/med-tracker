# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationInteractionLookup do
  fixtures :locations, :medications

  it 'maps known interactions between a lookup result and accessible medication stock' do
    result = NhsDmd::SearchResult.new(
      code: '3183411000001109',
      display: 'Warfarin 1mg tablets',
      system: 'https://dmd.nhs.uk',
      concept_class: 'VMP'
    )

    interactions = described_class.new(medication_scope: Medication.where(id: medications(:aspirin).id)).call(result)

    expect(interactions).to contain_exactly(
      hash_including(
        severity: 'high',
        severity_label: 'High',
        interacting_medication_name: 'Aspirin',
        description: a_string_including('bleeding')
      )
    )
  end

  it 'returns no warnings when interaction data is missing' do
    result = NhsDmd::SearchResult.new(
      code: '999',
      display: 'Cetirizine 10mg tablets',
      system: 'https://dmd.nhs.uk',
      concept_class: 'VMP'
    )

    expect(described_class.new(medication_scope: Medication.none).call(result)).to eq([])
  end
end
