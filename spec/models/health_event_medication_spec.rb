# frozen_string_literal: true

require 'rails_helper'

RSpec.describe HealthEventMedication do
  fixtures :people, :medications

  it 'snapshots the linked medication name' do
    event = HealthEvent.create!(
      person: people(:john),
      event_kind: :suspected_side_effect,
      title: 'Nausea',
      started_on: Date.new(2026, 2, 1)
    )

    link = described_class.create!(health_event: event, medication: medications(:paracetamol))

    expect(link.medication_name).to eq('Paracetamol')
  end

  it 'prevents duplicate live medication links for an event' do
    event = HealthEvent.create!(
      person: people(:john),
      event_kind: :suspected_side_effect,
      title: 'Nausea',
      started_on: Date.new(2026, 2, 1)
    )
    described_class.create!(health_event: event, medication: medications(:paracetamol))

    duplicate = described_class.new(health_event: event, medication: medications(:paracetamol))

    expect(duplicate).not_to be_valid
    expect(duplicate.errors[:medication_id]).to include('has already been linked to this health event')
  end
end
