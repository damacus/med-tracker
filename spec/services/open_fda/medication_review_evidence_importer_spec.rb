# frozen_string_literal: true

require 'rails_helper'

RSpec.describe OpenFda::MedicationReviewEvidenceImporter do
  let(:client) { instance_double(OpenFda::DrugLabelClient, labels: [label]) }
  let(:label) do
    {
      'set_id' => 'importer-spec-record',
      'effective_time' => '20250617',
      'version' => '4',
      'drug_interactions' => ['Narrative interaction evidence from the public label.'],
      'openfda' => {
        'brand_name' => ['Warfarin Sodium'],
        'generic_name' => ['WARFARIN SODIUM'],
        'substance_name' => ['WARFARIN SODIUM'],
        'pharm_class_epc' => ['Vitamin K Antagonist [EPC]']
      }
    }
  end

  it 'stores imported labels locally as unknown and unreviewed' do
    expect do
      described_class.new(client: client, retrieved_on: Date.new(2026, 7, 9)).call(limit: 80)
    end.to change(MedicationReviewEvidenceRecord, :count).by(1)

    expect(MedicationReviewEvidenceRecord.last).to have_attributes(expected_imported_attributes)
    expect(client).to have_received(:labels).with(limit: 80)
  end

  it 'imports every available snapshot label by default' do
    described_class.new(client: client, retrieved_on: Date.new(2026, 7, 9)).call

    expect(client).to have_received(:labels).with(limit: nil)
  end

  it 'updates the same source record instead of duplicating it' do
    importer = described_class.new(client: client, retrieved_on: Date.new(2026, 7, 9))

    importer.call(limit: 80)
    importer.call(limit: 80)

    expect(MedicationReviewEvidenceRecord.where(source_record_id: label.fetch('set_id')).count).to eq(1)
  end

  def expected_imported_attributes
    {
      source_name: 'openFDA / DailyMed SPL',
      source_record_id: 'importer-spec-record',
      source_version: '4', source_effective_on: Date.new(2025, 6, 17),
      product_name: 'Warfarin Sodium', active_ingredient: 'WARFARIN SODIUM', label_section: 'Drug Interactions',
      risk_level: 'unknown', match_confidence: 'unknown', match_status: 'unreviewed',
      candidate_terms: ['warfarin sodium'], pharmacologic_classes: ['vitamin k antagonist'], interacting_terms: []
    }
  end
end
