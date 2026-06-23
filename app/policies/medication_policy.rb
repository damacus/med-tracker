# frozen_string_literal: true

class MedicationPolicy < ApplicationPolicy
  def index?
    active_membership?
  end

  def show?
    medication_visible_in_household?
  end

  alias dosages? show?

  def create?
    medication_create_allowed_in_household?
  end

  alias new? create?

  def update?
    household_manager? && same_household?(record)
  end

  alias edit? update?

  def refill?
    can_refill_in_household?
  end

  def mark_as_ordered?
    refill?
  end

  def mark_as_received?
    refill?
  end

  def destroy?
    household_manager? && same_household?(record)
  end

  def finder?
    create? || refill?
  end

  class Scope < ApplicationPolicy::Scope
    def resolve
      household_medication_scope
    end

    private

    def household_medication_scope
      return scope.none unless active_membership?

      household_scope = scope.where(household: household)
      return household_scope if household_manager? || any_person_grant_allows?(:manage)

      household_scope.where(id: granted_medication_ids_for(:view))
    end
  end

  private

  def medication_visible_in_household?
    return false unless active_membership? && same_household?(record)
    return true if household_manager?

    granted_medication_ids_for(:view).include?(record.id)
  end

  def can_refill_in_household?
    return false unless active_membership?
    return household_manager? || any_person_grant_allows?(:view) if record.is_a?(Class)

    medication_visible_in_household?
  end

  def medication_create_allowed_in_household?
    return false unless household_manager? || any_person_grant_allows?(:manage)
    return true if record.is_a?(Class) || record.location_id.blank?

    same_household?(record.location)
  end
end
