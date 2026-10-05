# frozen_string_literal: true

require 'rails_helper'

RSpec.describe OpenFda::EvidenceAttributes do
  it 'maps a snapshot label into an unsaved evidence record attribute set' do
    attributes = described_class.new(retrieved_on: Date.new(2026, 7, 10)).call(
      'selection_term' => 'example', 'set_id' => 'set-id', 'version' => '5', 'effective_time' => '20260701',
      'drug_interactions' => ['Monitor use with warfarin.'],
      'openfda' => {
        'generic_name' => ['Example'], 'substance_name' => ['EXAMPLE'],
        'pharm_class_epc' => ['Example Class [EPC]']
      }
    )

    expect(attributes).to include(
      source_record_id: 'set-id', source_version: '5', source_effective_on: Date.new(2026, 7, 1),
      candidate_terms: ['example'], pharmacologic_classes: ['example class']
    )
  end

  it 'does not assign a combination co-ingredient or its classes to the selected medicine identity' do
    attributes = described_class.new(retrieved_on: Date.new(2026, 7, 10)).call(
      'selection_term' => 'aspirin', 'set_id' => 'aspirin-combination', 'version' => '2',
      'effective_time' => '20260701', 'drug_interactions' => ['Interaction evidence.'],
      'openfda' => {
        'generic_name' => ['ASPIRIN AND EXTENDED-RELEASE DIPYRIDAMOLE'],
        'substance_name' => %w[ASPIRIN DIPYRIDAMOLE],
        'pharm_class_epc' => ['Platelet Aggregation Inhibitor [EPC]']
      }
    )

    expect(attributes).to include(candidate_terms: ['aspirin'], pharmacologic_classes: [])
  end
end
