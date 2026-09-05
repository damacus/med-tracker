# frozen_string_literal: true

# Encapsulates the domain logic for recording a medication dose.
#
# Both Schedule and PersonMedication can be the source of a dose. This service
# handles the shared flow so controllers remain thin.
#
# @param source [Schedule, PersonMedication] the source of the dose
# @param amount_override [String, nil] optional dose amount override
# @param taken_from_medication_id [Integer, nil] optional specific medication ID
# @param user [User] the user recording the dose
# @param options [Hash] additional options, such as :taken_at (defaults to now)
# @return [MedicationAdministration::RecordDose::Result] object containing success boolean,
#   the take record, and any error symbol (:out_of_stock, :cooldown,
#   :invalid_amount, :selection_required, :invalid_source, :create_failed)
module MedicationAdministration
  class RecordDose
    include RecordDoseObservability

    Result = Data.define(:success, :take, :error)
    FUTURE_TOLERANCE = 60.minutes
    PreparedTake = Data.define(
      :source, :amount, :unit, :medication, :taken_at, :client_uuid, :error, :decision_context
    ) do
      def record
        source.medication_takes.create(medication_take_attributes)
      end

      private

      def medication_take_attributes
        {
          taken_at: taken_at,
          dose_amount: amount,
          dose_unit: unit,
          client_uuid: client_uuid,
          taken_from_medication: medication,
          taken_from_location: medication.location
        }
      end
    end

    def call(source:, amount_override:, taken_from_medication_id:, user:, **options)
      with_observability_attempt do
        publish_attempt(source:, user:, options:)
        record_with_household_lock(source:, amount_override:, taken_from_medication_id:, user:, options:)
      end
    rescue StandardError => e
      publish_unexpected_failure(source:, error: e)
      raise
    end

    private

    def record_with_household_lock(source:, **arguments)
      Households::LifecycleCutoffLock.with(household_id: source.household_id) do
        source_type = source_category(source)
        source = freshly_loaded_operational_source(source)
        return unavailable_household_failure(source_type:) unless source

        record_dose_with_savepoint(source:, **arguments)
      end
    end

    def record_dose_with_savepoint(**arguments)
      return record_dose(**arguments) unless ActiveRecord::Base.connection.transaction_open?

      ActiveRecord::Base.transaction(requires_new: true) { record_dose(**arguments) }
    end

    def record_dose(source:, amount_override:, taken_from_medication_id:, user:, options:)
      prepared_take = prepare_take(
        source: source,
        amount_override: amount_override,
        taken_from_medication_id: taken_from_medication_id,
        user: user,
        options: options
      )
      return rule_blocked_failure(prepared_take, source:, user:, options:) if prepared_take.error

      take = prepared_take.record
      return persistence_failure(source:, user:, options:) unless take.persisted?

      success(take, source:, user:, options:)
    end

    def freshly_loaded_operational_source(source)
      source.reload
      source if source.household&.operational?
    rescue ActiveRecord::RecordNotFound
      nil
    end

    def failure(error)
      Result.new(success: false, take: nil, error: error)
    end

    def rule_blocked_failure(prepared_take, source:, user:, options:)
      publish_rule_blocked_metric(prepared_take, source:, user:, options:)
      failure(prepared_take.error)
    end

    def persistence_failure(source:, user:, options:)
      publish_take_metric('take_errors.med_tracker', source:, user:, options:, error: :create_failed)
      failure(:create_failed)
    end

    def success(take, source:, user:, options:)
      publish_take_metric('take_recorded.med_tracker', source:, user:, options:)
      Observability::MedicationTransactionOutcome.register(
        dose_payload: dose_taken_payload(take),
        source_category: source_category(source),
        context: Current.observability_context
      )
      Result.new(success: true, take: take, error: nil)
    end

    def prepare_take(source:, amount_override:, taken_from_medication_id:, user:, options:)
      return prepared_error(:future_taken_at) if future_taken_at?(options.fetch(:taken_at, Time.current))

      prepare_valid_take(source:, amount_override:, taken_from_medication_id:, user:, options:)
    end

    def prepare_valid_take(source:, amount_override:, taken_from_medication_id:, user:, options:)
      taken_at = options.fetch(:taken_at, Time.current)
      resolver = MedicationStockSourceResolver.new(user: user, source: source, taken_at: taken_at)
      return prepared_error(resolver.blocked_reason) if resolver.blocked_reason

      amount = normalize_amount(amount_override.presence || default_dose_amount_for(source, taken_at))
      return prepared_error(:invalid_amount) if invalid_amount?(amount)

      error, medication = resolve_stock_source(resolver, taken_from_medication_id)
      return prepared_error(error) if error

      decision_error = overlapping_decision_error(source, taken_at)
      return decision_error if decision_error

      prepared_success(source:, amount:, medication:, taken_at:, options:)
    end

    def overlapping_decision_error(source, taken_at)
      decision_context = MedicationDoseDecisionContext.new(source: source, taken_at: taken_at)
      return unless decision_context.blocked?

      prepared_error(decision_context.blocked_reason, decision_context: decision_context.audit_payload)
    end

    def future_taken_at?(taken_at)
      taken_at > Time.current + FUTURE_TOLERANCE
    end

    def prepared_error(error, decision_context: nil)
      PreparedTake.new(
        source: nil, amount: nil, unit: nil, medication: nil,
        taken_at: nil, client_uuid: nil, error: error, decision_context: decision_context
      )
    end

    def prepared_success(source:, amount:, medication:, taken_at:, options:)
      PreparedTake.new(
        source: source,
        amount: amount,
        unit: default_dose_unit_for(source, taken_at),
        medication: medication,
        taken_at: taken_at,
        client_uuid: options[:client_uuid],
        error: nil,
        decision_context: nil
      )
    end

    def resolve_stock_source(resolver, taken_from_medication_id)
      return [:selection_required, nil] if resolver.selection_required?(taken_from_medication_id)

      medication = resolver.resolve_selected(taken_from_medication_id)
      return [:invalid_source, nil] if medication.blank?

      [nil, medication]
    end

    def normalize_amount(raw)
      return nil if raw.blank?

      BigDecimal(raw.to_s)
    rescue ArgumentError
      nil
    end

    def default_dose_amount_for(source, taken_at)
      return source.effective_dose_amount(effective_date(taken_at)) if source.respond_to?(:effective_dose_amount)

      source.default_dose_amount
    end

    def default_dose_unit_for(source, taken_at)
      return source.effective_dose_unit(effective_date(taken_at)) if source.respond_to?(:effective_dose_unit)

      source.dose_unit
    end

    def effective_date(taken_at)
      return taken_at.to_date if taken_at.respond_to?(:to_date)

      Time.zone.today
    end

    def invalid_amount?(amount)
      amount.nil? || amount <= 0
    end
  end
end
