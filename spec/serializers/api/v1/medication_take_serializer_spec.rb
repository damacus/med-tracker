# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Api::V1::MedicationTakeSerializer do
  it 'serialises source, event and subject data' do
    take = create(:medication_take, :for_schedule, client_uuid: SecureRandom.uuid)
    expect(described_class.new(take).as_json).to include(
      id: take.id, client_uuid: take.client_uuid, schedule_id: take.schedule_id,
      person_medication_id: take.person_medication_id,
      taken_from_medication_id: take.taken_from_medication_id,
      taken_from_location_id: take.taken_from_location_id,
      dose_amount: take.dose_amount&.to_f, dose_unit: take.dose_unit,
      taken_at: take.taken_at&.iso8601, updated_at: take.updated_at.iso8601,
      person_id: take.person&.id, medication_id: take.medication&.id
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
    expect(json[:schedule_id]).to be_nil
  end
end
