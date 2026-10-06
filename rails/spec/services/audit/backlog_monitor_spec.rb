# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Audit::BacklogMonitor do
  it 'emits only aggregate warning and critical backlog data' do
    now = Time.iso8601('2026-07-09T13:00:00Z')
    deliveries = [
      instance_double(AuditExportDelivery, created_at: now - 10.minutes),
      instance_double(AuditExportDelivery, created_at: now - 70.minutes)
    ]
    events, subscription = capture_notifications

    result = described_class.new(deliveries:, now:).call

    expect(result).to have_attributes(severity: 'critical', pending_count: 2, oldest_age_seconds: 4200)
    expect(events).to contain_exactly(
      severity: 'critical', pending_count: 2, oldest_age_seconds: 4200,
      warning_threshold_seconds: 300, critical_threshold_seconds: 3600
    )
    expect(events.first.keys).not_to include(:household_id, :source_id, :event_type)
  ensure
    ActiveSupport::Notifications.unsubscribe(subscription) if subscription
  end

  it 'reports healthy when the outbox has no overdue entries' do
    now = Time.iso8601('2026-07-09T13:00:00Z')
    delivery = instance_double(AuditExportDelivery, created_at: now - 1.minute)

    result = described_class.new(deliveries: [delivery], now:).call

    expect(result.severity).to eq('healthy')
  end

  def capture_notifications
    events = []
    subscription = ActiveSupport::Notifications.subscribe('audit_delivery_backlog.med_tracker') do |event|
      events << event.payload
    end
    [events, subscription]
  end
end
