# frozen_string_literal: true

class MedicationStockSourceResolver
  attr_reader :user, :source, :taken_at

  def initialize(user:, source:, taken_at: Time.current)
    @user = user
    @source = source
    @taken_at = taken_at
  end

  def available_medications
    @available_medications ||= matching_medications.reject(&:out_of_stock?)
  end

  def blocked_reason
    return :out_of_stock if available_medications.empty?
    return :cooldown unless medication_plan_can_take?

    nil
  end

  def resolve_selected(taken_from_medication_id)
    return available_medications.first if taken_from_medication_id.blank? && available_medications.one?

    medication = matching_medications.find do |candidate|
      candidate.id == taken_from_medication_id.to_i
    end

    return if medication.blank? || medication.out_of_stock?

    medication
  end

  def selection_required?(taken_from_medication_id)
    taken_from_medication_id.blank? && available_medications.many?
  end

  private

  def matching_medications
    @matching_medications ||= begin
      medication = source.medication
      resolved_scope
        .joins(:location)
        .includes(:location)
        .where(
          name: medication.name,
          dosage_amount: medication.dosage_amount,
          dosage_unit: medication.dosage_unit
        )
        .order('locations.name ASC, medications.id ASC')
        .to_a
    end
  end

  def resolved_scope
    return Medication.where(id: source.medication_id) if user.blank?

    MedicationPolicy::Scope.new(user, Medication.all).resolve
  end

  def medication_plan_can_take?
    return source.can_take_at?(taken_at) unless source.respond_to?(:dose_constraints)
    return true unless source.dose_constraints.restrictions?

    DoseTimingPolicy.new(
      takes: medication_plan_takes,
      dose_constraints: source.dose_constraints,
      dose_cycle: source_dose_cycle
    ).can_take_at?(taken_at)
  end

  def medication_plan_takes
    MedicationTake
      .left_joins(:schedule, :person_medication)
      .where(taken_at: 31.days.ago.beginning_of_day..taken_at)
      .where(
        '(schedules.person_id = :person_id AND schedules.medication_id = :medication_id) OR ' \
        '(person_medications.person_id = :person_id AND person_medications.medication_id = :medication_id)',
        person_id: source.person_id,
        medication_id: source.medication_id
      )
      .to_a
  end

  def source_dose_cycle
    source.respond_to?(:dose_cycle) ? source.dose_cycle : 'daily'
  end
end
