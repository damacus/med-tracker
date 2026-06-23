# frozen_string_literal: true

class CarerRelationshipPolicy < ApplicationPolicy
  def index?
    household_manager?
  end

  def show?
    household_manager? || person_grant_allows?(record.patient, :view)
  end

  def create?
    return true if household_manager?

    assign_dependent?
  end

  alias new? create?

  def update?
    household_manager? && relationship_in_household?
  end

  alias edit? update?

  def destroy?
    household_manager? && relationship_in_household?
  end

  def activate?
    household_manager? && relationship_in_household?
  end

  def assign_dependent?
    dependent_patient? && (household_manager? || person_grant_allows?(record.patient, :manage))
  end

  private

  def dependent_patient?
    return false unless record.respond_to?(:patient)

    record.patient&.person_type.in?(%w[minor dependent_adult]) && record.patient.has_capacity == false
  end

  def relationship_in_household?
    household.present? && record.patient&.household_id == household.id && record.carer&.household_id == household.id
  end

  class Scope < ApplicationPolicy::Scope
    def resolve
      return scope.none unless active_membership?
      return household_relationship_scope if household_manager?

      household_relationship_scope.where(patient_id: granted_person_ids_for(:view))
    end

    private

    def household_relationship_scope
      person_ids = Person.where(household: household).select(:id)
      scope.where(patient_id: person_ids, carer_id: person_ids)
    end
  end
end
