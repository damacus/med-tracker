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
    return :paused if source.respond_to?(:paused?) && source.paused?
    return :out_of_stock if available_medications.empty?
    return :cooldown unless source.can_take_at?(taken_at)

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
          dose_amount: medication.dose_amount,
          dose_unit: medication.dose_unit
        )
        .order('locations.name ASC, medications.id ASC')
        .to_a
    end
  end

  def resolved_scope
    context = authorization_context
    return Medication.where(id: source.medication_id) unless context

    MedicationPolicy::Scope.new(context, Medication.all).resolve
  end

  def authorization_context
    return user if user.is_a?(AuthorizationContext)
    return AuthorizationContext.current if AuthorizationContext.current
    return unless user.respond_to?(:person)

    account = user.person&.account
    membership = account&.first_active_household_membership
    return unless membership

    AuthorizationContext.new(account: account, household: membership.household, membership: membership)
  end
end
