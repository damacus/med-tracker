# frozen_string_literal: true

module Observability
  module JobCompletionSubscriber
    module_function

    def install
      @install ||= ActiveSupport::Notifications.subscribe('perform.active_job', method(:call))
    end

    def call(notification)
      payload = notification.payload
      job = payload.fetch(:job)
      error = payload[:exception_object]
      Publisher.publish(JobEvent.from(**event_options(job, error, notification.duration)))
      record_notification_failure(job, error) if error
    rescue StandardError => e
      EmergencyDiagnostic.write(error: e, event_name: 'job.completed')
    end

    def record_notification_failure(job, error)
      kind = NotificationStage.kind_for_job(job)
      return unless kind

      NotificationStage.emit(
        kind:,
        stage: :job_execution,
        reason: :job_failed,
        error_type: error.class
      )
    end
    private_class_method :record_notification_failure

    def event_options(job, error, duration)
      {
        job_class: job.class.name,
        job_id: job.job_id,
        queue_name: job.queue_name,
        arguments: job.arguments,
        outcome: error ? :failure : :success,
        duration_ms: duration,
        error_type: error&.class
      }
    end
    private_class_method :event_options
  end
end
