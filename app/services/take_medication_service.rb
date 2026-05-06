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
  PreparedTake = Data.define(:source, :amount, :unit, :medication, :taken_at, :client_uuid, :error) do
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
    prepared_take = prepare_take(
      source: source,
      amount_override: amount_override,
      taken_from_medication_id: taken_from_medication_id,
      user: user,
      options: options
    )
    return failure(prepared_take.error) if prepared_take.error

    take = prepared_take.record
    return failure(:create_failed) unless take.persisted?

    success(take)
  end

  private

  def failure(error)
    Result.new(success: false, take: nil, error: error)
  end

  def success(take)
    publish_dose_taken(take)
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

    prepared_success(source:, amount:, medication:, taken_at:, options:)
  end

  def prepared_error(error)
    PreparedTake.new(
      source: nil, amount: nil, unit: nil, medication: nil,
      taken_at: nil, client_uuid: nil, error: error
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
      error: nil
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
