# frozen_string_literal: true

class MedicationStockSourceResolver
  attr_reader :user, :source, :taken_at

  def initialize(user:, source:, taken_at: Time.current, matching_medications: nil)
    @user = user
    @source = source
    @taken_at = taken_at
    @matching_medications = matching_medications
  end

  def preload(sources)
    context = authorization_context
    assignments = stock_assignments(sources, context)
    stock = batch_stock(sources, context).group_by { |medication| stock_signature(medication) }
    sources.index_with do |candidate|
      matches = batch_matches(candidate, stock, assignments, context)
      self.class.new(user: user, source: candidate, taken_at: taken_at, matching_medications: matches)
    end
  end

  def available_medications
    @available_medications ||= matching_medications.reject(&:out_of_stock?)
  end

  def blocked_reason
    return :paused if source.respond_to?(:paused?) && source.paused?
    return :inactive if outside_schedule?
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

  def batch_matches(candidate, stock, assignments, context)
    matches = stock.fetch(stock_signature(candidate.medication), [])
    return matches.select { |medication| medication.id == candidate.medication_id } unless context
    return matches unless assignments

    ids = assignments.fetch(candidate.person_id, []) + [candidate.medication_id]
    matches.select { |medication| ids.include?(medication.id) }
  end

  def stock_signature(medication)
    [medication.name, medication.dose_amount, medication.dose_unit]
  end

  def batch_stock(sources, context)
    scope = if context
              MedicationPolicy::Scope.new(context, Medication.all).resolve
            else
              Medication.where(id: sources.map(&:medication_id))
            end
    scope.joins(:location).includes(:location).order('locations.name ASC, medications.id ASC').to_a
  end

  def stock_assignments(sources, context)
    return unless context && !household_manager_context?(context)

    people = sources.map(&:person_id).uniq
    schedules = Schedule.where(household: context.household, person_id: people).pluck(:person_id, :medication_id)
    direct = PersonMedication.where(household: context.household, person_id: people).pluck(:person_id, :medication_id)
    (schedules + direct).group_by(&:first).transform_values { |pairs| pairs.map(&:last).uniq }
  end

  def outside_schedule?
    source.respond_to?(:applies_on?) && !source.applies_on?(taken_at.to_date)
  end

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

    policy_scope = MedicationPolicy::Scope.new(context, Medication.all).resolve
    return policy_scope if household_manager_context?(context)

    policy_scope.where(id: source_person_medication_ids(context))
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

  def household_manager_context?(context)
    membership = context.membership
    return true if membership&.active? && (membership.owner? || membership.administrator?)

    platform_support_context?(context)
  end

  def platform_support_context?(context)
    context.account&.platform_admin&.active? &&
      Current.support_access_session&.active? &&
      Current.support_access_session.household_id == context.household&.id
  end

  def source_person_medication_ids(context)
    person_id = source_person_id
    return [source.medication_id] if person_id.blank?

    ids = [source.medication_id]
    ids.concat(Schedule.where(household: context.household, person_id: person_id).pluck(:medication_id))
    ids.concat(PersonMedication.where(household: context.household, person_id: person_id).pluck(:medication_id))
    ids.compact.uniq
  end

  def source_person_id
    return source.person_id if source.respond_to?(:person_id) && source.person_id.present?
    return source.person&.id if source.respond_to?(:person)

    nil
  end
end
