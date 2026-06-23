# frozen_string_literal: true

class PersonMedicationPolicy < ApplicationPolicy
  def index?
    active_membership?
  end

  def show?
    person_medication_person_grant_allows?(:view)
  end

  def create?
    person_medication_create_allowed_in_household?
  end

  alias new? create?

  def update?
    person_medication_person_grant_allows?(:manage)
  end

  alias edit? update?

  def destroy?
    person_medication_person_grant_allows?(:manage)
  end

  def take_medication?
    person_medication_person_grant_allows?(:record)
  end

  private

  def person_id_for_authorization
    record.respond_to?(:person) && record.person ? record.person.id : nil
  end

  def person_medication_person_grant_allows?(access_level)
    return household_manager? if record.is_a?(Class)

    person_grant_allows?(record.person, access_level)
  end

  def person_medication_create_allowed_in_household?
    return false unless person_medication_person_grant_allows?(:manage)
    return true if record.is_a?(Class) || record.medication_id.blank?

    same_household?(record.medication)
  end

  class Scope < ApplicationPolicy::Scope
    def resolve
      household_person_medication_scope
    end

    private

    def household_person_medication_scope
      return scope.none unless active_membership?

      scope.where(household: household, person_id: granted_person_ids_for(:view))
    end
  end
end
