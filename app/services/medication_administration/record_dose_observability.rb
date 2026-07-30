# frozen_string_literal: true

module MedicationAdministration
  module RecordDoseObservability
    private

    def with_observability_attempt
      previous = Current.observability_context
      root = previous || Observability::CorrelationContext.start
      Current.observability_context = root.next_attempt
      yield
    ensure
      Current.observability_context = previous
    end

    def publish_attempt(source:, user:, options:)
      event = publish_take_metric('take_attempted.med_tracker', source:, user:, options:)
      return unless event

      Current.observability_context = Current.observability_context.caused_by(
        event.to_h.fetch('event.id'),
        preserve_attempt: true
      )
    end

    def unavailable_household_failure(source_type:)
      Observability::Publisher.emit(
        name: :medication_take_failed,
        outcome: :failure,
        severity: :warn,
        reason: :household_unavailable,
        attributes: { source_category: source_type }
      )
      failure(:household_unavailable)
    end

    def publish_unexpected_failure(source:, error:)
      Observability::Publisher.emit(
        name: :medication_take_failed,
        outcome: :failure,
        severity: :error,
        reason: :unexpected_failure,
        attributes: { source_category: source_category(source) },
        error_type: error.class
      )
    end

    def publish_take_metric(event_name, source:, user:, options:, error: nil)
      Observability::DomainEventPublisher.instrument(
        event_name,
        take_metric_payload(source:, user:, options:, error:, decision_context: nil)
      )
    end

    def publish_rule_blocked_metric(prepared_take, source:, user:, options:)
      Observability::DomainEventPublisher.instrument(
        'take_blocked_by_rules.med_tracker',
        take_metric_payload(
          source:,
          user:,
          options:,
          error: prepared_take.error,
          decision_context: prepared_take.decision_context
        )
      )
    end

    def take_metric_payload(source:, user:, options:, error:, decision_context:)
      {
        environment: Rails.env.to_s,
        role: metric_role(user),
        route: options[:route],
        medicine_context_class: source.class.name,
        source_type: source.class.model_name.singular,
        error: error&.to_s
      }.merge(decision_context || {})
    end

    def metric_role(user)
      return user.membership&.role if user.is_a?(AuthorizationContext)
      return unless user.respond_to?(:person)

      account = user.person&.account
      account&.first_active_household_membership&.role
    end

    def source_category(source)
      source.class.model_name.singular.to_sym
    end

    def dose_taken_payload(take)
      {
        take_id: take.id,
        source_type: take.source_type,
        source_id: take.source_record_id,
        person_id: take.person&.id,
        medication_id: take.medication&.id,
        inventory_medication_id: take.inventory_medication&.id,
        inventory_location_id: take.inventory_location&.id,
        dose_amount: take.dose_amount&.to_f,
        dose_unit: take.dose_unit,
        taken_at: take.taken_at
      }
    end
  end
end
