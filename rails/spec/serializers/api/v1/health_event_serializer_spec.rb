# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Api::V1::HealthEventSerializer do
  fixtures :people, :medications

  it 'serialises person and medication portable identities when present' do
    event = HealthEvent.create!(
      person: people(:jane),
      event_kind: :suspected_side_effect,
      title: 'Nausea',
      started_on: Date.new(2026, 7, 7)
    )
    HealthEventMedication.create!(health_event: event, medication: medications(:paracetamol))

    json = described_class.new(event).as_json

    expect(json).to include(
      person_portable_id: people(:jane).portable_id,
      medication_ids: [medications(:paracetamol).id],
      medication_portable_ids: [medications(:paracetamol).portable_id]
    )
  end

  it 'serialises missing optional date and association values as nil or empty arrays' do
    updated_at = Time.zone.parse('2026-07-07 12:00:00')
    json = described_class.new(missing_optional_event(updated_at)).as_json

    expect(json).to include(
      person_portable_id: nil,
      started_on: nil,
      ended_on: nil,
      medication_portable_ids: []
    )
  end

  def missing_optional_event(updated_at)
    instance_double(
      HealthEvent,
      id: 1, portable_id: SecureRandom.uuid, person_id: nil, person: nil,
      event_kind: 'illness', severity: nil, title: 'Cold', notes: nil,
      started_on: nil, ended_on: nil, updated_at: updated_at,
      medication_ids: [], medications: []
    )
  end
end
