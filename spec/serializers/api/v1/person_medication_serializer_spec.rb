# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Api::V1::PersonMedicationSerializer do
  it 'serialises association, schedule and dosing-limit data' do
    pm = create(:person_medication, dose_cycle: :daily)
    json = described_class.new(pm).as_json
    expect(json).to include(
      id: pm.id, portable_id: pm.portable_id, person_id: pm.person_id,
      person_portable_id: pm.person.portable_id, medication_id: pm.medication_id,
      medication_portable_id: pm.medication.portable_id,
      dose_unit: pm.dose_unit, dose_cycle: 'daily',
      administration_kind: pm.administration_kind, notes: pm.notes, position: pm.position,
      active: true, paused: false,
      max_daily_doses: pm.max_daily_doses, min_hours_between_doses: pm.min_hours_between_doses,
      updated_at: pm.updated_at.iso8601
    )
    expect(json[:dose_amount]).to eq(pm.dose_amount.to_f)
  end

  it 'serialises dose_amount as a Float (calls to_f on the stored decimal)' do
    pm = create(:person_medication, dose_cycle: :weekly)
    json = described_class.new(pm).as_json
    expect(json[:dose_amount]).to be_a(Float)
    expect(json[:dose_cycle]).to eq('weekly')
  end

  it 'serialises notes as a non-nil value when present' do
    pm = create(:person_medication, notes: 'After meals')
    expect(described_class.new(pm).as_json[:notes]).to eq('After meals')
  end

  it 'serialises dosing limits when set' do
    pm = create(:person_medication, :with_both_restrictions)
    json = described_class.new(pm).as_json
    expect(json[:max_daily_doses]).to eq(2)
    expect(json[:min_hours_between_doses]).to eq(12)
  end

  it 'serialises paused assignments with paused true and active false' do
    pm = create(:person_medication, active: false)
    json = described_class.new(pm).as_json

    expect(json[:active]).to be false
    expect(json[:paused]).to be true
  end
end
