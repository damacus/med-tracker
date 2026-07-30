# frozen_string_literal: true

module Observability
  module DomainEventPublisher
    STAGES = {
      'take_attempted.med_tracker' => :take_attempted,
      'take_recorded.med_tracker' => :take_recorded,
      'take_blocked_by_rules.med_tracker' => :take_blocked_by_rules,
      'take_errors.med_tracker' => :take_errors,
      'dose_taken.med_tracker' => :dose_taken,
      'low_stock_threshold_reached.med_tracker' => :low_stock_threshold_reached,
      'audit_delivery_backlog.med_tracker' => :audit_delivery_backlog,
      'rack_attack.throttled' => :rack_attack_throttled
    }.freeze

    module_function

    def instrument(name, payload = nil, **attributes)
      payload = (payload || {}).merge(attributes)
      stage = STAGES.fetch(name)
      publication = safe_emit(name:, **payload)
      ActiveSupport::Notifications.instrument(name, payload)
      publication
    rescue StandardError => e
      safe_emit(
        name: :domain_event_subscriber_failed,
        outcome: :failure,
        severity: :error,
        reason: :subscriber_failed,
        attributes: { workflow_stage: stage },
        error_type: e.class
      )
      raise
    end

    def safe_emit(**)
      Publisher.emit(**)
    rescue StandardError
      nil
    end
    private_class_method :safe_emit
  end
end
