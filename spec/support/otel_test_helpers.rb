# frozen_string_literal: true

# Test helpers for OpenTelemetry verification
module OtelTestHelpers
  extend ActiveSupport::Concern

  included do
    before :each do
      clear_test_exporters!
    end
  end

  class_methods do
    def with_otel_test_exporter
      let(:test_span_exporter) { OpenTelemetry::SDK::Trace::Export::InMemorySpanExporter.new }

      before :each do
        configure_test_exporters(test_span_exporter)
      end
    end
  end

  def configure_test_exporters(span_exporter)
    # Reconfigure OpenTelemetry for testing with in-memory exporter
    OpenTelemetry::SDK.shutdown if OpenTelemetry.tracer_provider.is_a?(OpenTelemetry::SDK::Trace::TracerProvider)

    OpenTelemetry::SDK.configure do |c|
      c.service_name = 'medtracker-test'
      c.service_version = 'test'

      c.add_span_processor(
        OpenTelemetry::SDK::Trace::Export::BatchSpanProcessor.new(
          exporter: span_exporter
        )
      )

      c.use_all(
        'OpenTelemetry::Instrumentation::Rails' => {},
        'OpenTelemetry::Instrumentation::ActiveRecord' => {},
        'OpenTelemetry::Instrumentation::Rack' => {
          untraced_endpoints: ['/up', '/health', '/healthz', '/ready', '/live'],
          record_frontend_span: true
        }
      )
    end
  end

  def captured_spans
    test_span_exporter.finished_spans
  end

  def span_with_name(name)
    captured_spans.find { |span| span.name == name }
  end

  def clear_test_exporters!
    return unless defined?(test_span_exporter)

    test_span_exporter&.reset
  end

  def wait_for_spans_exported(timeout: 1)
    sleep(timeout)
    # Force flush span processor
    OpenTelemetry.tracer_provider.force_flush if OpenTelemetry.tracer_provider.respond_to?(:force_flush)
  end
end
