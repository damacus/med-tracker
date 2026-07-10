# frozen_string_literal: true

module Audit
  class BacklogMonitor
    Result = Data.define(:severity, :pending_count, :oldest_age_seconds)
    WARNING_AGE = 5.minutes
    CRITICAL_AGE = 1.hour

    def initialize(deliveries: AuditExportDelivery.pending, now: Time.current)
      @deliveries = deliveries
      @now = now
    end

    def call
      ages = deliveries.map { |delivery| [(now - delivery.created_at).to_i, 0].max }
      result = Result.new(severity: severity(ages.max.to_i), pending_count: ages.size,
                          oldest_age_seconds: ages.max.to_i)
      ActiveSupport::Notifications.instrument('audit_delivery_backlog.med_tracker', notification_payload(result))
      result
    end

    private

    attr_reader :deliveries, :now

    def severity(age)
      return 'critical' if age >= CRITICAL_AGE
      return 'warning' if age >= WARNING_AGE

      'healthy'
    end

    def notification_payload(result)
      {
        severity: result.severity, pending_count: result.pending_count,
        oldest_age_seconds: result.oldest_age_seconds,
        warning_threshold_seconds: WARNING_AGE.to_i, critical_threshold_seconds: CRITICAL_AGE.to_i
      }
    end
  end
end
