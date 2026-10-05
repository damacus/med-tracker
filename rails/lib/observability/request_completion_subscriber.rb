# frozen_string_literal: true

module Observability
  module RequestCompletionSubscriber
    HEALTH_ROUTES = %w[/up /health /healthz /ready /live].freeze

    module_function

    def install
      @install ||= ActiveSupport::Notifications.subscribe('process_action.action_controller', method(:call))
    end

    def call(notification)
      event = request_event(notification)
      Publisher.publish(event) if event
    rescue StandardError => e
      EmergencyDiagnostic.write(error: e, event_name: 'http.request.completed')
    end

    def request_event(notification)
      payload = notification.payload
      request = payload.fetch(:request)
      status = (payload[:status] || (500 if payload[:exception_object])).to_i
      route = request.route_uri_pattern || rodauth_route(payload)
      return if HEALTH_ROUTES.include?(route) && status < 400

      RequestEvent.from(
        method: request.request_method, route:, status:, duration_ms: notification.duration,
        request_id: request.request_id, error_type: payload[:exception_object]&.class
      )
    end
    private_class_method :request_event

    def rodauth_route(payload)
      return unless payload[:controller] == 'RodauthController'

      action = payload[:action].to_s
      return unless action.match?(/\A[a-z_]{1,64}\z/)

      "/#{action.tr('_', '-')}"
    end
    private_class_method :rodauth_route
  end
end
