# frozen_string_literal: true

class OfflineController < ApplicationController
  def show
    render(Components::Offline::Shell.new)
  end

  def snapshot
    render(
      json: {
        data: snapshot_payload,
        meta: {generated_at: Time.current.iso8601}
      }
    )
  end

  def medication_takes
    attributes = queued_take_params
    client_uuid = attributes[:client_uuid]
    source_type = attributes[:source_type]
    source_id = attributes[:source_id]
    taken_at = attributes[:taken_at]
    dose_amount = attributes[:dose_amount]
    taken_from_medication_id = attributes[:taken_from_medication_id]

    if client_uuid.present?
      existing_take = policy_scope(MedicationTake).find_by(client_uuid: client_uuid)
      return render_synced_take(existing_take, status: :ok) if existing_take
    end

    source = offline_take_source(source_type, source_id)
    authorize(source, :take_medication?)

    parsed_taken_at = parse_taken_at(taken_at)
    return render_unprocessable(t("take_medications.invalid_taken_at")) if parsed_taken_at.blank?
    if parsed_taken_at > Time.current + TakeMedicationGuardable::FUTURE_TOLERANCE
      return render_unprocessable(t("take_medications.future_taken_at"))
    end

    result = TakeMedicationService.new.call(
      source: source,
      amount_override: dose_amount,
      taken_from_medication_id: taken_from_medication_id,
      user: current_user,
      taken_at: parsed_taken_at,
      client_uuid: client_uuid
    )

    return render_take_failure(result.error) unless result.success

    render_synced_take(result.take, status: :created)
  rescue ActiveRecord::RecordNotFound, Pundit::NotAuthorizedError
    render_unprocessable(t(".source_unavailable", default: "Queued dose source is no longer available."))
  end

  private

  def snapshot_payload
    {
      people: serialized(policy_scope(Person).includes(:locations, :notification_preference), Api::V1::PersonSerializer),
      locations: serialized(policy_scope(Location), Api::V1::LocationSerializer),
      medications: serialized(policy_scope(Medication).includes(:location), Api::V1::MedicationSerializer),
      schedules: serialized(policy_scope(Schedule).includes(:person, :medication), Api::V1::ScheduleSerializer),
      person_medications: serialized(
        policy_scope(PersonMedication).includes(:person, :medication),
        Api::V1::PersonMedicationSerializer
      ),
      medication_takes: serialized(recent_medication_takes, Api::V1::MedicationTakeSerializer)
    }
  end

  def serialized(records, serializer)
    records.map { |record| serializer.new(record).as_json }
  end

  def recent_medication_takes
    policy_scope(MedicationTake)
      .where(taken_at: 30.days.ago..Time.current.end_of_day)
      .includes(
        [
          {schedule: %i[person medication]},
          {person_medication: %i[person medication]},
          :taken_from_location,
          :taken_from_medication
        ]
      )
      .order(taken_at: :desc)
  end

  def queued_take_params
    params.permit(
      :client_uuid,
      :source_type,
      :source_id,
      :taken_at,
      :dose_amount,
      :dose_unit,
      :taken_from_medication_id
    )
  end

  def offline_take_source(source_type, source_id)
    case source_type
    when "schedule"
      policy_scope(Schedule).find(source_id)
    when "person_medication"
      policy_scope(PersonMedication).find(source_id)
    else
      raise ActiveRecord::RecordNotFound
    end
  end

  def parse_taken_at(value)
    Time.zone.parse(value.to_s)
  rescue ArgumentError, TypeError
    nil
  end

  def render_synced_take(take, status:)
    render(json: {data: Api::V1::MedicationTakeSerializer.new(take).as_json}, status: status)
  end

  def render_take_failure(error)
    render_unprocessable(take_failure_message(error))
  end

  def take_failure_message(error)
    case error
    when :out_of_stock
      t("take_medications.out_of_stock", default: "Cannot take medication: out of stock")
    when :cooldown
      t("take_medications.cooldown", default: "Cannot take medication: timing restrictions not met")
    when :selection_required
      t("take_medications.location_required", default: "Choose a location to record this dose.")
    when :invalid_source
      t("take_medications.invalid_location", default: "Selected location is unavailable for this medication.")
    when :invalid_amount
      t("person_medications.invalid_dose_configured", default: "Invalid dose configured")
    else
      t("take_medications.failure")
    end
  end

  def render_unprocessable(message)
    render(json: {error: {code: "unprocessable_content", message: message}}, status: :unprocessable_content)
  end
end
