# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Observability::Publisher do
  before do
    allow(Observability::CanonicalLogger).to receive(:write)
    allow(Observability::EmergencyDiagnostic).to receive(:write)
  end

  it 'writes one immutable canonical event' do
    event = described_class.emit(
      name: :medication_take_attempted,
      outcome: :unknown,
      severity: :info,
      reason: :requested,
      attributes: { source_category: :schedule }
    )

    expect(Observability::CanonicalLogger).to have_received(:write).with(event).once
    expect(event.to_h.fetch('event.id')).to eq(JSON.parse(event.to_json).fetch('event.id'))
  end

  it 'maps a frozen Active Support event before writing' do
    event = described_class.emit(
      name: 'rack_attack.throttled',
      throttle: 'medication_lookup/ip',
      ip: '203.0.113.42'
    )

    expect(event.to_h).to include(
      'event.name' => 'request.throttled',
      'medtracker.throttle.category' => 'medication_lookup'
    )
    expect(event.to_json).not_to include('203.0.113.42')
  end

  it 'returns nil and invokes one emergency diagnostic when writing fails' do
    allow(Observability::CanonicalLogger).to receive(:write).and_raise(IOError, 'stdout unavailable')

    result = described_class.emit(
      name: :medication_take_attempted,
      outcome: :unknown,
      severity: :info,
      reason: :requested
    )

    expect(result).to be_nil
    expect(Observability::EmergencyDiagnostic).to have_received(:write).once
  end

  it 'keeps suppressing nested writes until the outer write finishes' do
    writes = 0
    allow(Observability::CanonicalLogger).to receive(:write) do
      writes += 1
      next unless writes == 1

      2.times { emit_event }
    end

    emit_event

    expect(Observability::CanonicalLogger).to have_received(:write).once
  end

  def emit_event
    described_class.emit(
      name: :medication_take_attempted,
      outcome: :unknown,
      severity: :info,
      reason: :requested
    )
  end
end
