# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Observability::OperationalEvent do
  let(:clock) { -> { Time.iso8601('2026-07-30T09:30:00Z') } }
  let(:service) do
    {
      name: 'medtracker',
      version: 'sha256:verified-image',
      environment: 'production'
    }
  end

  it 'emits one flat ECS-compatible application event envelope' do
    event = build_event

    expect(event.to_h).to include(
      '@timestamp' => '2026-07-30T09:30:00.000Z',
      'log.level' => 'info',
      'service.name' => 'medtracker',
      'service.version' => 'sha256:verified-image',
      'service.environment' => 'production',
      'event.name' => 'medication_take.attempted',
      'event.outcome' => 'unknown',
      'event.dataset' => 'medtracker.application',
      'medtracker.schema.version' => 1,
      'medtracker.reason' => 'requested',
      'medtracker.source.category' => 'schedule'
    )
    expect(event.to_h.fetch('message', nil)).not_to be_a(Hash)
  end

  it 'generates collision-resistant event identifiers that are never reused' do
    identifiers = Array.new(1_000) { build_event.to_h.fetch('event.id') }

    expect(identifiers.uniq.size).to eq(identifiers.size)
    expect(identifiers).to all(match(/\A[0-9a-f]{8}-(?:[0-9a-f]{4}-){3}[0-9a-f]{12}\z/))
  end

  it 'keeps one event identifier stable through repeated serialization' do
    event = build_event
    first = JSON.parse(event.to_json)
    retry_payload = JSON.parse(event.to_json)

    expect(retry_payload.fetch('event.id')).to eq(first.fetch('event.id'))
    expect(retry_payload).to eq(first)
  end

  it 'creates a new event identifier for a new application emission' do
    first = build_event(event_id: '43d1e299-df0d-4c7f-89d0-61706b895442')
    second = first.reemit

    expect(second.to_h.fetch('event.id')).not_to eq(first.to_h.fetch('event.id'))
  end

  it 'enforces stable outcomes, severities, and allowlisted failure reasons' do
    expect do
      build_event(severity: :verbose, outcome: :maybe, reason: 'raw exception text')
    end.to raise_error(ArgumentError)
  end

  it 'includes safe structured failure identity without exception text' do
    event = build_event(
      severity: :error,
      outcome: :failure,
      reason: :persistence_failed,
      error_type: ActiveRecord::RecordNotSaved
    )

    expect(event.to_h).to include(
      'log.level' => 'error',
      'event.outcome' => 'failure',
      'medtracker.reason' => 'persistence_failed',
      'error.type' => 'ActiveRecord::RecordNotSaved'
    )
    expect(event.to_json).not_to include('exception.message')
  end

  def build_event(**overrides)
    Observability::OperationalEvent.build(
      name: :medication_take_attempted,
      outcome: :unknown,
      severity: :info,
      reason: :requested,
      attributes: { source_category: :schedule },
      service:,
      clock:,
      **overrides
    )
  end
end
