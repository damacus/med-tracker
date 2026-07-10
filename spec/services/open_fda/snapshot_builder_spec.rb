# frozen_string_literal: true

require 'rails_helper'

RSpec.describe OpenFda::SnapshotBuilder do
  let(:manifest) { instance_double(OpenFda::SnapshotManifest, version: 3, selection: %w[warfarin ibuprofen]) }
  let(:client) do
    instance_double(
      OpenFda::DrugLabelClient,
      labels_for: {
        'meta' => { 'last_updated' => '2026-07-10' },
        'results' => [label('warfarin-id', 'Warfarin'), label('ibuprofen-id', 'Ibuprofen')]
      }
    )
  end

  it 'builds a stable minimal snapshot with source metadata' do
    snapshot = described_class.new(client: client, manifest: manifest, generated_on: Date.new(2026, 7, 10)).call

    expect(snapshot).to include(
      'selection_version' => 3,
      'generated_on' => '2026-07-10',
      'openfda_last_updated' => '2026-07-10'
    )
    expect(snapshot.fetch('labels')).to contain_exactly(
      hash_including('set_id' => 'warfarin-id', 'selection_term' => 'warfarin',
                     'openfda' => include('substance_name')),
      hash_including('set_id' => 'ibuprofen-id', 'selection_term' => 'ibuprofen',
                     'openfda' => include('substance_name'))
    )
    expect(client).to have_received(:labels_for).with(%w[warfarin ibuprofen])
  end

  def label(set_id, ingredient)
    {
      'set_id' => set_id,
      'id' => "#{set_id}-version",
      'effective_time' => '20260701',
      'version' => '4',
      'drug_interactions' => ["Interaction evidence for #{ingredient}."],
      'openfda' => {
        'brand_name' => [ingredient],
        'generic_name' => [ingredient],
        'substance_name' => [ingredient.upcase],
        'pharm_class_epc' => ['Example Class [EPC]'],
        'rxcui' => ['123']
      },
      'warnings' => ['Not retained in the minimal snapshot.']
    }
  end
end
