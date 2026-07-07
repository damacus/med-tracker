# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Api::V1::DosageOptionSerializer do
  it 'serialises medication portable identity when present' do
    dosage_option = create(:dosage)
    json = described_class.new(dosage_option).as_json

    expect(json).to include(
      medication_id: dosage_option.medication_id,
      medication_portable_id: dosage_option.medication.portable_id
    )
  end

  it 'serialises missing medication association as a nil portable ID' do
    updated_at = Time.zone.parse('2026-07-07 12:00:00')
    json = described_class.new(missing_medication_dosage(updated_at)).as_json

    expect(json[:medication_portable_id]).to be_nil
    expect(json[:updated_at]).to eq(updated_at.iso8601)
  end

  def missing_medication_dosage(updated_at)
    instance_double(
      MedicationDosageOption,
      id: 1, portable_id: SecureRandom.uuid, medication_id: nil, medication: nil,
      amount: 5, unit: 'ml', frequency: 'daily', description: nil,
      default_for_adults: false, default_for_children: false,
      default_max_daily_doses: nil, default_min_hours_between_doses: nil,
      default_dose_cycle: 'daily', current_supply: nil, reorder_threshold: nil,
      updated_at: updated_at
    )
  end
end
