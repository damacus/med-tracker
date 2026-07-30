# frozen_string_literal: true

require_relative 'span_sanitizer'

module Otel
  class SpanSanitizingProcessor
    def initialize
      @sanitizer = SpanSanitizer.new
    end

    def on_start(span, _parent_context)
      sanitize_span_attributes(span)
    end

    def on_finish(_span); end

    def force_flush(**_)
      OpenTelemetry::SDK::Trace::Export::SUCCESS
    end

    def shutdown(**_)
      OpenTelemetry::SDK::Trace::Export::SUCCESS
    end

    private

    def sanitize_span_attributes(span)
      return unless span.respond_to?(:attributes) && span.attributes

      sanitized = @sanitizer.sanitize_attributes(span.attributes)
      sanitized.each do |key, value|
        span.set_attribute(key, value) if value != span.attributes[key]
      end
    rescue StandardError => e
      Observability::DiagnosticEvent.emit(
        component: :span_sanitizer,
        reason: :operation_failed,
        severity: :warn,
        error: e
      )
    end
  end
end
