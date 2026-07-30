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

  it 'starts an opaque workflow and enqueues a no-argument canary job' do
    allow(ObservabilityCanaryJob).to receive(:perform_later)

    described_class.run

    expect(ObservabilityCanaryJob).to have_received(:perform_later).with(no_args)
    expect(Observability::Publisher).to have_received(:emit)
    expect(Current.observability_context).to be_nil
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
