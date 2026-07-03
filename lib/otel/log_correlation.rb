# frozen_string_literal: true

module Otel
  class LogCorrelation
    class << self
      def options(span: OpenTelemetry::Trace.current_span)
        return {} unless span&.context&.valid?

        {
          'trace.id' => span.context.hex_trace_id,
          'span.id' => span.context.hex_span_id
        }
      end
    end
  end
end
