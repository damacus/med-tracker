# frozen_string_literal: true

# Concern for adding OpenTelemetry custom spans to model operations
module OtelInstrumented
  extend ActiveSupport::Concern

  class_methods do
    # Get the tracer for this model
    def otel_tracer
      @otel_tracer ||= OpenTelemetry.tracer_provider.tracer(
        "medtracker.#{name.underscore}",
        '1.0.0'
      )
    end
  end

  included do
    after_create :trace_create
    after_update :trace_update
    after_destroy :trace_destroy
  end

  private

  def trace_create
    trace_operation('create')
  end

  def trace_update
    trace_operation('update')
  end

  def trace_destroy
    trace_operation('destroy')
  end

  def trace_operation(operation)
    self.class.otel_tracer.in_span(
      "#{self.class.name.underscore}.#{operation}",
      attributes: otel_span_attributes(operation),
      kind: :internal
    ) do |span|
      span.add_event("#{self.class.name} #{operation}d")
    end
  rescue StandardError => e
    Rails.logger.warn "[OpenTelemetry] Failed to trace #{operation}: #{e.message}"
  end

  # Override in models to provide custom attributes
  def otel_span_attributes(operation)
    {
      'model.name' => self.class.name,
      'model.id' => id.to_s,
      'model.operation' => operation
    }
  end
end
