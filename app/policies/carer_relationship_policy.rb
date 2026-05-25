# frozen_string_literal: true

class CarerRelationshipPolicy < ApplicationPolicy
  def index?
    admin_or_clinician?
  end

  def show?
    admin_or_clinician? || carer_owns_relationship?
  end

  def create?
    admin?
  end

  alias new? create?

  def update?
    admin?
  end

  alias edit? update?

  def destroy?
    admin?
  end

  def activate?
    admin?
  end

  def assign_dependent?
    dependent_patient? && (admin? || parent_owns_dependent?)
  end

  private

  def carer_owns_relationship?
    (user&.person && record.carer_id == user.person.id) || false
  end

  def parent_owns_dependent?
    return false unless user&.parent? && user.person_id && record.patient_id

    active_patient_relationships
      .joins(:patient)
      .where(patient_id: record.patient_id)
      .exists?(people: { person_type: %i[minor dependent_adult], has_capacity: false })
  end

  def dependent_patient?
    record.patient&.person_type.in?(%w[minor dependent_adult]) && record.patient.has_capacity == false
  end

  class Scope < ApplicationPolicy::Scope
    def resolve
      return scope.none unless user
      return scope.all if admin_or_clinician?

      scope.where(carer_id: user.person_id)
    end
  end
end
