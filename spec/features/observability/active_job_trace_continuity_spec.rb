# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Active Job trace continuity' do
  include ActiveJob::TestHelper

  let(:tracer) { OpenTelemetry.tracer_provider.tracer('medtracker-active-job-trace-spec') }
  let(:span_processor) { OpenTelemetry::SDK::Trace::Export::SimpleSpanProcessor.new(exporter) }
  let(:exporter) do
    Class.new do
      attr_reader :finished_spans

      def initialize
        @finished_spans = []
      end

      def export(span_data, timeout: nil)
        @finished_spans.concat(span_data)
        @timeout = timeout
        OpenTelemetry::SDK::Trace::Export::SUCCESS
      end

      def force_flush(timeout: nil)
        @timeout = timeout
        OpenTelemetry::SDK::Trace::Export::SUCCESS
      end

      def shutdown(timeout: nil)
        @timeout = timeout
        OpenTelemetry::SDK::Trace::Export::SUCCESS
      end
    end.new
  end

  around do |example|
    original_adapter = ActiveJob::Base.queue_adapter
    ActiveJob::Base.queue_adapter = :test
    OpenTelemetry.tracer_provider.add_span_processor(span_processor)

    example.run
  ensure
    clear_enqueued_jobs
    clear_performed_jobs
    ActiveJob::Base.queue_adapter = original_adapter
    OpenTelemetry.tracer_provider.instance_variable_get(:@span_processors).delete(span_processor)
  end

  it 'links a performed job to the trace that enqueued it' do
    stub_const('TraceContinuityJob', Class.new(ApplicationJob) do
      def perform; end
    end)
    request_trace_id = nil

    tracer.in_span('request') do |span|
      request_trace_id = span.context.trace_id
      TraceContinuityJob.perform_later
    end
    perform_enqueued_jobs

    expect(process_span.links.map { |link| link.span_context.trace_id }).to include(request_trace_id)
  end

  it 'records an escaped job failure on the consumer span' do
    stub_const('FailingTraceJob', Class.new(ApplicationJob) do
      def perform
        raise 'job failure'
      end
    end)

    tracer.in_span('request') { FailingTraceJob.perform_later }

    expect { perform_enqueued_jobs }.to raise_error(RuntimeError, 'job failure')
    expect(process_span.status.code).to eq(OpenTelemetry::Trace::Status::ERROR)
    expect(process_span.events.map(&:name)).to include('exception')
  end

  def process_span
    exporter.finished_spans.find { |span| span.name.end_with?(' process') }
  end
end
