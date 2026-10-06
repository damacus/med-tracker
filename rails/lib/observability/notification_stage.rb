# frozen_string_literal: true

module Observability
  module NotificationStage
    UNKNOWN_OUTCOME_REASONS = %i[
      provider_accepted delivery_unknown retrying past_occurrence suppressed duplicate
    ].freeze
    FAILURE_REASONS = %i[
      job_failed partial_failure permanent_failure household_unavailable person_unavailable
    ].freeze
    ATTRIBUTE_KEYS = %i[
      notification_kind workflow_stage notification_channel notification_provider recipient_count
    ].freeze
    JOB_KINDS = {
      'MedicationReminderJob' => :dose_due,
      'MissedDoseNotificationJob' => :missed_dose,
      'LowStockNotificationJob' => :low_stock
    }.freeze

    module_function

    def emit(kind:, stage:, reason:, **attributes)
      options = {
        name: :notification_stage,
        outcome: outcome(reason),
        severity: severity(reason),
        reason:,
        attributes: safe_attributes(kind:, stage:, attributes:),
        error_type: attributes[:error_type]
      }.compact
      Publisher.emit(**options)
    end

    def kind_for_job(job)
      JOB_KINDS[job.class.name]
    end

    def outcome(reason)
      return :failure if FAILURE_REASONS.include?(reason.to_sym)
      return :unknown if UNKNOWN_OUTCOME_REASONS.include?(reason.to_sym)

      :success
    end
    private_class_method :outcome

    def severity(reason)
      return :error if reason.to_sym == :job_failed
      return :warn if FAILURE_REASONS.include?(reason.to_sym)

      :info
    end
    private_class_method :severity

    def safe_attributes(kind:, stage:, attributes:)
      {
        notification_kind: kind,
        workflow_stage: stage,
        notification_channel: attributes[:channel],
        notification_provider: attributes[:provider],
        recipient_count: attributes[:recipient_count]
      }.compact.slice(*ATTRIBUTE_KEYS)
    end
    private_class_method :safe_attributes
  end
end
