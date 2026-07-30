# frozen_string_literal: true

module Observability
  module OperationalEventPayloadBuilder
    private

    def build_payload(options)
      envelope(options).compact
                       .merge(safe_attributes(options.fetch(:name), options.fetch(:attributes)))
                       .merge(safe_context(options.fetch(:context)))
    end

    def envelope(options)
      timestamp_fields(options)
        .merge(service_fields(options))
        .merge(event_fields(options))
        .merge(error_fields(options))
    end

    def timestamp_fields(options)
      {
        '@timestamp' => options.fetch(:clock).call.utc.iso8601(3),
        'log.level' => options.fetch(:severity).to_s,
        'message' => safe_message(options[:message])
      }
    end

    def service_fields(options)
      service = options.fetch(:service)
      {
        'service.name' => service.fetch(:name),
        'service.version' => service.fetch(:version),
        'service.environment' => service.fetch(:environment)
      }
    end

    def event_fields(options)
      {
        'event.name' => self.class::EVENT_NAMES.fetch(options.fetch(:name).to_sym),
        'event.outcome' => options.fetch(:outcome).to_s,
        'event.dataset' => options.fetch(:dataset),
        'event.id' => options.fetch(:event_id),
        'medtracker.schema.version' => self.class::SCHEMA_VERSION,
        'medtracker.reason' => options.fetch(:reason).to_s
      }
    end

    def error_fields(options)
      { 'error.type' => safe_error_type(options[:error_type] || options[:error]&.class) }
    end
  end
end
