# frozen_string_literal: true

require 'rails_helper'

RSpec.describe ObservabilityCanaryJob do
  let(:tracer) { instance_double(OpenTelemetry::Trace::Tracer) }
  let(:counter) { double(add: nil) }
  let(:emitted_kinds) { [] }
  let(:emitted_contexts) { [] }

  before do
    allow(tracer).to receive(:in_span).with('observability.canary').and_yield
    allow(Observability::DeployedCanary).to receive_messages(tracer:, counter:)
    allow(Observability::Publisher).to receive(:emit) do |**options|
      emitted_kinds << options.fetch(:attributes).fetch(:canary_kind)
      emitted_contexts << Current.observability_context.to_event_fields
      :event
    end
  end

  it 'emits the application and job canaries in the collected worker process' do
    described_class.perform_now

    expect(emitted_kinds).to eq(%i[application_event job])
    expect_workflow_and_attempts
  end

  def expect_workflow_and_attempts
    expect_shared_workflow
    expect_distinct_attempts
  end

  def expect_shared_workflow
    expect(workflow_ids).to all(eq(workflow_ids.first))
    expect(workflow_ids.first).to be_present
  end

  def expect_distinct_attempts
    expect(attempt_ids).to eq(attempt_ids.uniq)
  end

  def workflow_ids
    emitted_contexts.map { |fields| fields.fetch('medtracker.workflow.id') }
  end

  def attempt_ids
    emitted_contexts.map { |fields| fields.fetch('medtracker.attempt.id') }
  end
end
