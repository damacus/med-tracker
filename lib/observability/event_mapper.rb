# frozen_string_literal: true

module Observability
  module EventMapper
    extend EventMappingSupport

    EVENT_NAMES = %w[
      take_attempted.med_tracker
      take_recorded.med_tracker
      take_blocked_by_rules.med_tracker
      take_errors.med_tracker
      dose_taken.med_tracker
      low_stock_threshold_reached.med_tracker
      audit_delivery_backlog.med_tracker
      rack_attack.throttled
    ].freeze
    SOURCE_CATEGORIES = {
      'schedule' => :schedule,
      'person_medication' => :person_medication
    }.freeze
    BLOCKED_REASONS = %w[
      out_of_stock cooldown paused invalid_amount selection_required invalid_source
    ].freeze
    BACKLOG_OUTCOMES = { 'healthy' => :success }.freeze
    BACKLOG_SEVERITIES = { 'healthy' => :info, 'warning' => :warn, 'critical' => :error }.freeze
    BACKLOG_REASONS = {
      'healthy' => :backlog_healthy,
      'warning' => :backlog_warning,
      'critical' => :backlog_critical
    }.freeze

    module_function

    def event_names
      EVENT_NAMES
    end

    def map(event_name, payload)
      raise ArgumentError, 'unregistered custom event' unless EVENT_NAMES.include?(event_name)

      send(:"map_#{event_name.tr('.', '_')}", payload.symbolize_keys)
    end

    def map_take_attempted_med_tracker(payload)
      event(:medication_take_attempted, :unknown, :info, :requested, source_attributes(payload))
    end
    private_class_method :map_take_attempted_med_tracker

    def map_take_recorded_med_tracker(payload)
      event(:medication_take_persisted, :unknown, :info, :provisional, source_attributes(payload))
    end
    private_class_method :map_take_recorded_med_tracker

    def map_take_blocked_by_rules_med_tracker(payload)
      reason = payload[:error].to_s
      reason = 'rules_blocked' unless BLOCKED_REASONS.include?(reason)
      event(:medication_take_blocked, :failure, :warn, reason.to_sym, source_attributes(payload))
    end
    private_class_method :map_take_blocked_by_rules_med_tracker

    def map_take_errors_med_tracker(payload)
      event(:medication_take_failed, :failure, :error, :persistence_failed, source_attributes(payload))
    end
    private_class_method :map_take_errors_med_tracker

    def map_dose_taken_med_tracker(payload)
      event(:medication_take_committed, :success, :info, :committed, source_attributes(payload))
    end
    private_class_method :map_dose_taken_med_tracker

    def map_low_stock_threshold_reached_med_tracker(payload)
      event(
        :low_stock_threshold_reached,
        :unknown,
        :info,
        :threshold_reached,
        source_attributes(payload)
      )
    end
    private_class_method :map_low_stock_threshold_reached_med_tracker

    def map_audit_delivery_backlog_med_tracker(payload)
      severity = payload[:severity].to_s
      event(
        :audit_delivery_backlog_evaluated,
        BACKLOG_OUTCOMES.fetch(severity, :unknown),
        BACKLOG_SEVERITIES.fetch(severity, :warn),
        BACKLOG_REASONS.fetch(severity, :backlog_warning),
        {
          backlog_severity: severity,
          pending_count: bounded_integer(payload[:pending_count]),
          oldest_age_seconds: bounded_integer(payload[:oldest_age_seconds])
        }
      )
    end
    private_class_method :map_audit_delivery_backlog_med_tracker

    def map_rack_attack_throttled(payload)
      category = payload[:throttle].to_s.split('/').first
      category = 'unknown' unless %w[medication_lookup ai_medication_suggestions].include?(category)
      event(
        :request_throttled,
        :failure,
        :warn,
        :throttled,
        { throttle_category: category.to_sym }
      )
    end
    private_class_method :map_rack_attack_throttled
  end
end
