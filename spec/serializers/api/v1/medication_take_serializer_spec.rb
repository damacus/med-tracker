# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Api::V1::MedicationTakeSerializer do
  it 'serialises source, event and subject data' do
    take = create(:medication_take, :for_schedule, client_uuid: SecureRandom.uuid)
    expect(described_class.new(take).as_json).to include(
      id: take.id, portable_id: take.portable_id, client_uuid: take.client_uuid,
      schedule_id: take.schedule_id, schedule_portable_id: take.schedule.portable_id,
      person_medication_id: take.person_medication_id, person_medication_portable_id: nil,
      taken_from_medication_id: take.taken_from_medication_id,
      taken_from_location_id: take.taken_from_location_id,
      dose_amount: take.dose_amount&.to_f, dose_unit: take.dose_unit,
      taken_at: take.taken_at&.iso8601, updated_at: take.updated_at.iso8601,
      person_id: take.person&.id, person_portable_id: take.person.portable_id,
      medication_id: take.medication&.id, medication_portable_id: take.medication.portable_id
    )
  end

  it 'includes a specific client_uuid value' do
    uuid = SecureRandom.uuid
    take = create(:medication_take, :for_schedule, client_uuid: uuid)
    expect(described_class.new(take).as_json[:client_uuid]).to eq(uuid)
  end

  it 'derives person_id from schedule' do
    take = create(:medication_take, :for_schedule)
    json = described_class.new(take).as_json
    expect(json[:person_id]).to eq(take.schedule.person_id)
  end

  it 'derives medication_id from schedule' do
    take = create(:medication_take, :for_schedule)
    json = described_class.new(take).as_json
    expect(json[:medication_id]).to eq(take.schedule.medication_id)
  end

  it 'serialises dose_amount as a Float' do
    take = create(:medication_take, :for_schedule)
    json = described_class.new(take).as_json
    expect(json[:dose_amount]).to be_a(Float)
  end

  it 'includes person_medication_id from person_medication source' do
    take = create(:medication_take, :for_person_medication)
    json = described_class.new(take).as_json
    expect(json[:person_medication_id]).to eq(take.person_medication_id)
    expect(json[:person_medication_portable_id]).to eq(take.person_medication.portable_id)
    expect(json[:schedule_id]).to be_nil
    expect(json[:schedule_portable_id]).to be_nil
  end

  it 'serialises missing optional associations as nil portable IDs' do
    json = described_class.new(missing_association_take).as_json

    expect(json).to include(
      schedule_portable_id: nil,
      person_medication_portable_id: nil,
      taken_from_medication_portable_id: nil,
      taken_from_location_portable_id: nil,
      dose_amount: nil,
      taken_at: nil,
      person_id: nil,
      person_portable_id: nil,
      medication_id: nil,
      medication_portable_id: nil
    )
  end

  def missing_association_take
    instance_double(
      MedicationTake,
      id: 1, portable_id: SecureRandom.uuid, client_uuid: nil,
      schedule_id: nil, schedule: nil, person_medication_id: nil,
      person_medication: nil, taken_from_medication_id: nil,
      taken_from_medication: nil, taken_from_location_id: nil,
      taken_from_location: nil, dose_amount: nil, dose_unit: nil,
      taken_at: nil, updated_at: Time.zone.parse('2026-07-07 12:00:00'),
      person: nil, medication: nil
    )
  end
end
