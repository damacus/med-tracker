# frozen_string_literal: true

module Observability
  module JobRetrySubscriber
    module_function

    def install
      @install ||= ActiveSupport::Notifications.subscribe('enqueue_retry.active_job', method(:call))
    end

    def call(notification)
      payload = notification.payload
      job = payload.fetch(:job)
      kind = NotificationStage.kind_for_job(job)
      return unless kind

      error = payload[:error]
      NotificationStage.emit(
        kind:,
        stage: :job_execution,
        reason: :retrying,
        error_type: error&.class
      )
    rescue StandardError => e
      EmergencyDiagnostic.write(error: e, event_name: 'job.retrying')
    end
  end
end
