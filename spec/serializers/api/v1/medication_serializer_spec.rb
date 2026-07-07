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

  it 'serialises medications without a location' do
    json = described_class.new(medication_without_location).as_json

    expect(json[:location_id]).to be_nil
    expect(json[:location_portable_id]).to be_nil
  end

  def medication_without_location
    instance_double(
      Medication,
      id: 1, portable_id: SecureRandom.uuid, name: 'Paracetamol',
      display_name: 'Paracetamol', category: 'tablet', description: nil,
      dose_amount: nil, dose_unit: nil, current_supply: nil,
      reorder_threshold: nil, reorder_status: 'ok', location_id: nil,
      location: nil, updated_at: Time.zone.parse('2026-07-07 12:00:00'),
      low_stock?: false, out_of_stock?: false,
      days_until_low_stock: nil, days_until_out_of_stock: nil
    )
  end
end
