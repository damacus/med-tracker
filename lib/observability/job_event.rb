# frozen_string_literal: true

module Observability
  module JobEvent
    module_function

    def from(**options)
      outcome = options.fetch(:outcome)
      OperationalEvent.build(
        name: :job_completed,
        outcome: outcome.to_sym,
        severity: severity(outcome),
        reason: outcome.to_sym == :failure ? :failed : :completed,
        dataset: 'medtracker.job',
        attributes: event_attributes(options),
        context: job_context(options.fetch(:context, ContextEnricher.call), options.fetch(:job_id)),
        error_type: options[:error_type]
      )
    end

    def event_attributes(options)
      {
        job_class: options.fetch(:job_class).to_s,
        job_queue: options[:queue_name].to_s,
        duration: (options.fetch(:duration_ms, 0).to_f * 1_000_000).round
      }
    end
    private_class_method :event_attributes

    def severity(outcome)
      outcome.to_sym == :failure ? :error : :info
    end
    private_class_method :severity

    def job_context(context, job_id)
      fields = context.respond_to?(:to_event_fields) ? context.to_event_fields : context.to_h
      fields.merge('medtracker.job.id' => job_id)
    end
    private_class_method :job_context
  end
end
