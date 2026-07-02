# frozen_string_literal: true

module Otel
  class ExceptionRecorder
    class << self
      def record(exception, source:)
        span = OpenTelemetry::Trace.current_span
        return unless span&.context&.valid?

        span.record_exception(exception, attributes: exception_attributes(exception, source))
        span.set_attribute('error.type', exception.class.name)
        span.set_attribute('exception.escaped', true)
        span.set_attribute('exception.source', source)
        span.status = OpenTelemetry::Trace::Status.error(exception.class.name)
      end

      private

      def exception_attributes(exception, source)
        {
          'error.type' => exception.class.name,
          'exception.escaped' => true,
          'exception.source' => source
        }
      end
    end
  end
end
