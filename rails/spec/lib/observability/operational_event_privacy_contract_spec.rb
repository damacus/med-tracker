# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Observability::OperationalEvent do
  let(:markers) do
    {
      health_data: 'Wellman Original for Daniel Webb',
      credential: 'password=correct-horse-battery-staple',
      token: 'Bearer token-secret-123',
      cookie: '_med_tracker_session=cookie-secret-456',
      network_identifier: '203.0.113.42',
      job_arguments: 'MissedDoseNotificationJob household=41 schedule=52 person=63',
      domain_identifier: 'medication_take_id=987654',
      schedule_timestamp: '2026-07-30T08:04:00+01:00',
      exception_text: 'Failed for Daniel Webb taking Wellman Original'
    }
  end

  it 'excludes every prohibited marker from application event serialization' do
    serialized = operational_event_class.build(
      name: :medication_take_failed,
      outcome: :failure,
      severity: :error,
      reason: :persistence_failed,
      attributes: markers,
      error: RuntimeError.new(markers.fetch(:exception_text))
    ).to_json

    markers.each_value { |marker| expect(serialized).not_to include(marker) }
    expect(JSON.parse(serialized)).to include(
      'error.type' => 'RuntimeError',
      'medtracker.reason' => 'persistence_failed'
    )
  end

  it 'does not serialize Active Job arguments into lifecycle events' do
    event = Observability::JobEvent.from(
      job_class: 'MissedDoseNotificationJob',
      job_id: '4a42ed43-4cf2-496e-a180-6829ce4efc06',
      arguments: markers.values,
      outcome: :success
    )

    markers.each_value { |marker| expect(event.to_json).not_to include(marker) }
  end

  it 'does not serialize raw request, cookie, query, or network values' do
    event = Observability::RequestEvent.from(
      method: 'GET',
      route: '/households/:household_slug/dashboard',
      status: 200,
      duration_ms: 12.5,
      request_id: 'request-opaque',
      query: markers.fetch(:health_data),
      cookies: markers.fetch(:cookie),
      remote_ip: markers.fetch(:network_identifier)
    )

    markers.each_value { |marker| expect(event.to_json).not_to include(marker) }
  end

  it 'keeps correlation identifiers out of metric labels' do
    labels = Observability::MetricLabels.build(
      workflow_id: '43d1e299-df0d-4c7f-89d0-61706b895442',
      event_id: '94b50dab-6035-478c-bb91-1dad7617ade5',
      causation_id: '310ece2e-3422-4606-97f5-228da5f9587f',
      attempt_id: '36e156c5-35bf-4a87-a2f0-398b65f59b48',
      outcome: :success,
      source_category: :schedule
    )

    expect(labels).to eq('outcome' => 'success', 'source_category' => 'schedule')
  end

  it 'keeps storage events aggregate and excludes protected migration inputs' do
    serialized = storage_event.to_json

    markers.each_value { |marker| expect(serialized).not_to include(marker) }
    expect(JSON.parse(serialized)).to include(
      'event.name' => 'storage.stage',
      'medtracker.storage.processed_count' => 7,
      'medtracker.storage.failed_count' => 1,
      'error.type' => 'RuntimeError'
    )
  end

  def storage_event
    operational_event_class.build(
      name: :storage_stage,
      outcome: :failure,
      severity: :error,
      reason: :failed,
      attributes: markers.merge(
        storage_operation: :migration,
        storage_backend: :dual,
        storage_phase: :persistent_with_s3_mirror,
        processed_count: 7,
        verified_count: 6,
        failed_count: 1
      ),
      error: RuntimeError.new(markers.fetch(:exception_text))
    )
  end

  def operational_event_class
    Observability::OperationalEvent
  end
end
