# frozen_string_literal: true

require 'rails_helper'

class ObservabilityContextCharacterizationJob < ApplicationJob
  def perform
    Current.observability_context.to_event_fields
  end
end

RSpec.describe ApplicationJob do
  it 'serializes only opaque workflow context and restores it while performing' do
    context = Observability::CorrelationContext.start.next_attempt
    job = ObservabilityContextCharacterizationJob.new
    job.observability_context = context

    serialized = job.serialize
    restored = ObservabilityContextCharacterizationJob.deserialize(serialized)

    expect(serialized.fetch('medtracker_observability_context')).to eq(context.to_propagation)
    expect(restored.perform_now).to eq(context.to_event_fields)
    expect(serialized.to_json).not_to include('person_id', 'medication_id', 'scheduled_time')
  end

  it 'inherits the workflow but rotates the attempt for a child job' do
    parent = Observability::CorrelationContext.start.next_attempt
    Current.observability_context = parent
    job = ObservabilityContextCharacterizationJob.new

    job.run_callbacks(:enqueue) { job.serialize }

    expect(job.observability_context.workflow_id).to eq(parent.workflow_id)
    expect(job.observability_context.attempt_id).not_to eq(parent.attempt_id)
  ensure
    Current.observability_context = nil
  end
end
