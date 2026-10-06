# frozen_string_literal: true

require 'rails_helper'

RSpec.describe LowStockNotificationJob do
  include ActiveJob::TestHelper

  around do |example|
    original_queue_adapter = ActiveJob::Base.queue_adapter
    ActiveJob::Base.queue_adapter = :test
    example.run
  ensure
    clear_enqueued_jobs
    ActiveJob::Base.queue_adapter = original_queue_adapter
    Current.observability_context = nil
  end

  it 'propagates the medication workflow into the low-stock job without domain identifiers' do
    context = Observability::CorrelationContext.start.next_attempt
    Current.observability_context = context

    Observability::DomainEventPublisher.instrument(
      'low_stock_threshold_reached.med_tracker',
      household_id: 42,
      medication_id: 43,
      take_id: 44,
      source_type: 'schedule'
    )

    job_data = enqueued_jobs.find { |entry| entry.fetch(:job) == described_class }
    propagation = job_data.fetch('medtracker_observability_context')

    expect(propagation.fetch('workflow.id')).to eq(context.workflow_id)
    expect(propagation.fetch('attempt.id')).not_to eq(context.attempt_id)
    expect(propagation.keys).to contain_exactly('workflow.id', 'causation.id', 'attempt.id', 'expires_at')
  end
end
