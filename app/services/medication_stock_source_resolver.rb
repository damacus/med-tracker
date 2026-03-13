# frozen_string_literal: true

class MedicationStockSourceResolver
  attr_reader :user, :source

  def initialize(user:, source:)
    @user = user
    @source = source
  end

  def available_medications
    @available_medications ||= matching_medications.reject(&:out_of_stock?)
  end

  def blocked_reason
    return :out_of_stock if available_medications.empty?

    can_take = block_given? ? yield : source.can_take_now?
    return :cooldown unless can_take

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
end
