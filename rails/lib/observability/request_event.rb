# frozen_string_literal: true

module Observability
  module RequestEvent
    module_function

    def from(**options)
      status = options.fetch(:status).to_i
      OperationalEvent.build(
        name: :http_request_completed,
        outcome: status >= 400 ? :failure : :success,
        severity: severity(status),
        reason: :completed,
        dataset: 'medtracker.request',
        attributes: event_attributes(options, status),
        context: request_context(options.fetch(:context, ContextEnricher.call), options.fetch(:request_id)),
        error_type: options[:error_type]
      )
    end

    def event_attributes(options, status)
      {
        http_method: options.fetch(:method).to_s.upcase,
        route: options.fetch(:route),
        status_code: status,
        duration: (options.fetch(:duration_ms).to_f * 1_000_000).round
      }
    end
    private_class_method :event_attributes

    def severity(status)
      return :error if status >= 500
      return :warn if status >= 400

      :info
    end
    private_class_method :severity

    def request_context(context, request_id)
      fields = context.respond_to?(:to_event_fields) ? context.to_event_fields : context.to_h
      fields.merge('medtracker.request.id' => request_id)
    end
    private_class_method :request_context
  end
end
