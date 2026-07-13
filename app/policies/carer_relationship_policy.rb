# frozen_string_literal: true

class CarerRelationshipPolicy < ApplicationPolicy
  def index?
    household_manager?
  end

  def show?
    relationship_in_household? && (household_manager? || person_grant_allows?(record.patient, :view))
  end

  def create?
    return household_manager? if class_record?

    relationship_in_household? && (household_manager? || assign_dependent?)
  end

  def new?
    return household_manager? if class_record?
    return household_manager? if record.patient.nil? && record.carer.nil?

    assign_dependent?
  end

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
    relationship_in_household? && dependent_patient? &&
      (household_manager? || person_grant_allows?(record.patient, :manage))
  end

  private

  def dependent_patient?
    return false unless record.respond_to?(:patient)

    record.patient&.person_type.in?(%w[minor dependent_adult]) && record.patient.has_capacity == false
  end

  def relationship_in_household?
    return false if household.blank?
    return false if class_record?

    record_household_id == household.id && carer_in_household?
  end

  def record_household_id
    record.household_id || record.patient&.household_id
  end

  def carer_in_household?
    record.carer.nil? || record.carer.household_id == household.id
  end

  def class_record?
    !record.respond_to?(:household_id)
  end

  class Scope < ApplicationPolicy::Scope
    def resolve
      return scope.none unless active_membership?
      return household_relationship_scope if household_manager?

      household_relationship_scope.where(patient_id: granted_person_ids_for(:view))
    end

    private

    def household_relationship_scope
      scope.where(household: household)
    end
  end
end
