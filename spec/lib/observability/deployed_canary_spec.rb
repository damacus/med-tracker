# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Observability::DeployedCanary do
  let(:tracer) do
    instance_double(OpenTelemetry::Trace::Tracer).tap do |tracer|
      allow(tracer).to receive(:in_span).with('observability.canary').and_yield
    end
  end
  let(:counter) do
    instance = Class.new do
      def add(*) end
    end.new
    allow(instance).to receive(:add)
    instance
  end

  before do
    allow(described_class).to receive_messages(tracer:, counter:)
    allow(Observability::Publisher).to receive(:emit).and_return(:event)
  end

  after { Current.observability_context = nil }

  it 'emits one safe application canary inside a retained trace and records a bounded metric' do
    context = Observability::CorrelationContext.start.next_attempt
    Current.observability_context = context

    expect(described_class.emit(kind: :application_event)).to eq(:event)

    expect(Observability::Publisher).to have_received(:emit).with(
      name: :observability_canary,
      outcome: :success,
      severity: :info,
      reason: :canary_emitted,
      attributes: { canary_kind: :application_event }
    )
    expect(counter).to have_received(:add).with(1, attributes: { 'canary.kind' => 'application_event' })
  end

  it 'starts an opaque workflow, enqueues a no-argument canary job, and flushes its enqueue trace' do
    trace_provider = instance_double(OpenTelemetry::SDK::Trace::TracerProvider, force_flush: :success)
    allow(ObservabilityCanaryJob).to receive(:perform_later)
    allow(OpenTelemetry).to receive(:tracer_provider).and_return(trace_provider)

    described_class.run

    expect(ObservabilityCanaryJob).to have_received(:perform_later).with(no_args)
    expect(tracer).to have_received(:in_span).with('observability.canary')
    expect(trace_provider).to have_received(:force_flush)
    expect(Current.observability_context).to be_nil
  end

  it 'does not let a trace flush failure prevent the canary job from being enqueued' do
    trace_provider = instance_double(OpenTelemetry::SDK::Trace::TracerProvider)
    allow(trace_provider).to receive(:force_flush).and_raise(RuntimeError, 'private exporter failure')
    allow(ObservabilityCanaryJob).to receive(:perform_later)

    allow(OpenTelemetry).to receive(:tracer_provider).and_return(trace_provider)

    expect { described_class.run }.not_to raise_error

    expect(ObservabilityCanaryJob).to have_received(:perform_later).with(no_args)
    expect(Current.observability_context).to be_nil
  end

  it 'does not let a failed trace flush result prevent the canary job from being enqueued' do
    trace_provider = instance_double(OpenTelemetry::SDK::Trace::TracerProvider, force_flush: :failure)
    allow(ObservabilityCanaryJob).to receive(:perform_later)
    allow(OpenTelemetry).to receive(:tracer_provider).and_return(trace_provider)

    expect { described_class.run }.not_to raise_error

    expect(trace_provider).to have_received(:force_flush)
    expect(ObservabilityCanaryJob).to have_received(:perform_later).with(no_args)
  end

  it 'uses a distinct attempt for each canary event while preserving the workflow' do
    context = Observability::CorrelationContext.start.next_attempt
    contexts = []
    Current.observability_context = context
    allow(Observability::Publisher).to receive(:emit) do
      contexts << Current.observability_context.to_event_fields
      :event
    end

    described_class.emit(kind: :application_event)
    described_class.emit(kind: :job)

    expect(contexts.map { |fields| fields.fetch('medtracker.workflow.id') }).to all(eq(context.workflow_id))
    attempts = contexts.map { |fields| fields.fetch('medtracker.attempt.id') }
    expect(attempts).to all(be_present)
    expect(attempts).to eq(attempts.uniq)
    expect(Current.observability_context).to eq(context)
  end

  it 'does not let a metric failure suppress the canonical canary' do
    allow(counter).to receive(:add).and_raise(RuntimeError, 'private exporter failure')
    allow(Observability::DiagnosticEvent).to receive(:emit)

    expect(described_class.emit(kind: :job)).to eq(:event)
    expect(Observability::DiagnosticEvent).to have_received(:emit).with(
      component: :observability_canary,
      reason: :operation_failed,
      severity: :warn,
      error: instance_of(RuntimeError)
    )
  end
end
