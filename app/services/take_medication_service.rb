# frozen_string_literal: true

# Encapsulates the domain logic for recording a medication dose.
#
# Both Schedule and PersonMedication can be the source of a dose. This service
# handles the shared flow so controllers remain thin.
#
# @example
#   result = TakeMedicationService.new.call(
#     source: @schedule,
#     amount_override: params[:dose_amount],
#     taken_from_medication_id: params[:taken_from_medication_id],
#     user: current_user,
#     taken_at: params[:taken_at] || Time.current   # optional, defaults to now
#   )
#   result.success  # => true / false
#   result.take     # => MedicationTake record (when successful)
#   result.error    # => :out_of_stock | :cooldown | :invalid_amount |
#                   #    :selection_required | :invalid_source | :create_failed
class TakeMedicationService
  Result = Data.define(:success, :take, :error)
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
    publish_take_metric('take_attempted.med_tracker', source:, user:, options:)
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

  private

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
    publish_dose_taken(take)
    publish_take_metric('take_recorded.med_tracker', source:, user:, options:)
    Result.new(success: true, take: take, error: nil)
  end

  def prepare_take(source:, amount_override:, taken_from_medication_id:, user:, options:)
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

  def publish_dose_taken(take)
    ActiveSupport::Notifications.instrument('dose_taken.med_tracker', dose_taken_payload(take))
  end

  def publish_take_metric(event_name, source:, user:, options:, error: nil)
    ActiveSupport::Notifications.instrument(
      event_name,
      take_metric_payload(source:, user:, options:, error:, decision_context: nil)
    )
  end

  def publish_rule_blocked_metric(prepared_take, source:, user:, options:)
    ActiveSupport::Notifications.instrument(
      'take_blocked_by_rules.med_tracker',
      take_metric_payload(
        source: source,
        user: user,
        options: options,
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
