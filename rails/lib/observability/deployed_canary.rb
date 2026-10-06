# frozen_string_literal: true

module Observability
  module DeployedCanary
    module_function

    def run
      previous_context = Current.observability_context
      Current.observability_context = CorrelationContext.start
      tracer.in_span('observability.canary') { ObservabilityCanaryJob.perform_later }
    ensure
      flush_trace_provider
      Current.observability_context = previous_context
    end

    def emit(kind:)
      previous_context = Current.observability_context
      Current.observability_context = (previous_context || CorrelationContext.start).next_attempt
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
    ensure
      Current.observability_context = previous_context
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

    def flush_trace_provider
      OpenTelemetry.tracer_provider.force_flush
    rescue StandardError
      nil
    end
    private_class_method :record_metric, :flush_trace_provider
  end
end
