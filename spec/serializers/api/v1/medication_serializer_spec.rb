# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Api::V1::MedicationSerializer do
  it 'keeps the canonical name and exposes the friendly display name' do
    medication = create(
      :medication,
      name: 'Movicol Paediatric Plain oral powder 6.9g sachets (Norgine Pharmaceuticals Ltd) 30 sachet 15 x 2 sachets',
      friendly_name: 'Movicol Paediatric Plain'
    )

    json = described_class.new(medication).as_json

    expect(json[:name]).to eq(
      'Movicol Paediatric Plain oral powder 6.9g sachets (Norgine Pharmaceuticals Ltd) 30 sachet 15 x 2 sachets'
    )
    expect(json[:display_name]).to eq('Movicol Paediatric Plain')
  end

  it 'includes portable identity fields for mobile clients' do
    medication = create(:medication)
    json = described_class.new(medication).as_json

    expect(json).to include(
      portable_id: medication.portable_id,
      location_portable_id: medication.location.portable_id
    )
  end
end
