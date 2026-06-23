# frozen_string_literal: true

module PolicyHelpers
  extend ActiveSupport::Concern

  private

  def authorization_context?
    context.is_a?(AuthorizationContext)
  end

  def account
    context&.account
  end

  def household
    context&.household
  end

  def membership
    context&.membership
  end

  def active_membership?
    membership&.active? || false
  end

  def household_owner?
    active_membership? && membership.owner?
  end

  def household_administrator?
    active_membership? && membership.administrator?
  end

  def household_manager?
    household_owner? || household_administrator?
  end

  def same_household?(record)
    household.present? && record.respond_to?(:household_id) && record.household_id == household.id
  end

  def person_grant_allows?(person, access_level)
    return false unless same_household?(person)
    return false unless active_membership?

    PersonAccessGrant.active
                     .where(household: household, household_membership: membership, person: person)
                     .any? { |grant| grant.cover_access?(access_level) }
  end

  def granted_person_ids_for(access_level)
    return Person.none.select(:id) unless active_membership?

    PersonAccessGrant.active
                     .where(
                       household: household,
                       household_membership: membership,
                       access_level: access_levels_covering(access_level)
                     )
                     .select(:person_id)
  end

  def any_person_grant_allows?(access_level)
    return false unless active_membership?

    PersonAccessGrant.active
                     .exists?(household: household,
                              household_membership: membership,
                              access_level: access_levels_covering(access_level))
  end

  def granted_medication_ids_for(access_level)
    return [] unless active_membership?

    person_ids = granted_person_ids_for(access_level)
    schedule_medication_ids = Schedule.where(household: household, person_id: person_ids).pluck(:medication_id)
    person_medication_ids = PersonMedication.where(household: household, person_id: person_ids).pluck(:medication_id)

    (schedule_medication_ids + person_medication_ids).compact.uniq
  end

  def access_levels_covering(access_level)
    requested_level = PersonAccessGrant::ACCESS_LEVEL_ORDER.fetch(access_level.to_s)

    PersonAccessGrant::ACCESS_LEVEL_ORDER.select { |_name, level| level >= requested_level }.keys
  end

  def admin? = household_manager?
end
