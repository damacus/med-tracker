# frozen_string_literal: true

module Observability
  module ContextEnricher
    module_function

    def call
      request_and_job_context
        .merge(correlation_context)
        .merge(Otel::LogCorrelation.options)
    end

    def request_and_job_context
      {
        'medtracker.request.id' => bounded_identifier(Current.request_id),
        'medtracker.job.id' => bounded_identifier(Current.job_id)
      }.compact
    end
    private_class_method :request_and_job_context

    def correlation_context
      Current.observability_context&.to_event_fields || {}
    end
    private_class_method :correlation_context

    def bounded_identifier(value)
      string = value.to_s
      return if string.blank? || string.bytesize > 128 || string.match?(/\s/)

      string
    end
    private_class_method :bounded_identifier
  end
end
