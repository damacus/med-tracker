# frozen_string_literal: true

require_relative 'span_sanitizer'

module Otel
  class SpanSanitizingProcessor
    def initialize
      @sanitizer = SpanSanitizer.new
    end

    def on_start(span, _parent_context)
      return unless span.respond_to?(:attributes) && span.attributes

      sanitized = @sanitizer.sanitize_attributes(span.attributes)
      sanitized.each do |key, value|
        span.set_attribute(key, value) if value != span.attributes[key]
      end
    rescue StandardError => e
      Rails.logger.warn "[OpenTelemetry] SpanSanitizingProcessor error: #{e.message}"
    end

    def on_finish(_span); end

    def force_flush(timeout: nil) # rubocop:disable Lint/UnusedMethodArgument
      OpenTelemetry::SDK::Trace::Export::SUCCESS
    end

    def shutdown(timeout: nil) # rubocop:disable Lint/UnusedMethodArgument
      OpenTelemetry::SDK::Trace::Export::SUCCESS
    end
  end
end
