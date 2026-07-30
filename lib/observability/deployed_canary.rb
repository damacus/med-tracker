# frozen_string_literal: true

module Observability
  module DeployedCanary
    module_function

    def run
      previous_context = Current.observability_context
      Current.observability_context = CorrelationContext.start.next_attempt
      emit(kind: :application_event)
      ObservabilityCanaryJob.perform_later
    ensure
      Current.observability_context = previous_context
    end

    def emit(kind:)
      tracer.in_span('observability.canary') do
        event = Publisher.emit(
          name: :observability_canary,
          outcome: :success,
          severity: :info,
          reason: :canary_emitted,
          attributes: { canary_kind: kind }
        )
        record_metric(kind)
        event
      end
    end

    def tracer
      @tracer ||= OpenTelemetry.tracer_provider.tracer('medtracker.observability_canary')
    end

    def counter
      @counter ||= OpenTelemetry.meter_provider
                                .meter('medtracker.observability_canary')
                                .create_counter(
                                  'medtracker.observability.canary',
                                  unit: '1',
                                  description: 'Safe deployed observability canary'
                                )
    end

    def record_metric(kind)
      counter.add(1, attributes: { 'canary.kind' => kind.to_s })
    rescue StandardError => e
      DiagnosticEvent.emit(
        component: :observability_canary,
        reason: :operation_failed,
        severity: :warn,
        error: e
      )
    end
    private_class_method :record_metric
  end
end
