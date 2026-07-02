# frozen_string_literal: true

require 'rails_helper'

RSpec.describe HealthEvent do
  fixtures :people

  it 'tracks ongoing notable illnesses' do
    event = described_class.new(
      person: people(:john),
      event_kind: :illness,
      title: 'Tonsillitis',
      started_on: Date.new(2026, 1, 4)
    )

    expect(event).to be_valid
    expect(event).to be_ongoing
  end

  it 'rejects an end date before the start date' do
    event = described_class.new(
      person: people(:john),
      event_kind: :suspected_side_effect,
      title: 'Rash',
      started_on: Date.new(2026, 1, 4),
      ended_on: Date.new(2026, 1, 3)
    )

    expect(event).not_to be_valid
    expect(event.errors[:ended_on]).to include('must be on or after the start date')
  end
end
