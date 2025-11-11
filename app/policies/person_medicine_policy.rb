# frozen_string_literal: true

class PersonMedicinePolicy < ApplicationPolicy
  def index?
    show?
  end

  def show?
    admin_or_clinician? || carer_with_patient?
  end

  def create?
    admin?
  end

  def new?
    create?
  end

  def update?
    admin?
  end

  def edit?
    update?
  end

  def destroy?
    admin?
  end

  def take_medicine?
    admin? || carer_with_patient?
  end

  private

  def carer_with_patient?
    (carer_or_parent? && user.person.patients.exists?(record.person.id)) || false
  end

  class Scope < ApplicationPolicy::Scope
    def resolve
      return scope.none unless user
      return scope.all if admin_or_clinician?
      return scope.none unless carer_or_parent?

      scope.where(person_id: accessible_person_ids)
    end

    private

    def accessible_person_ids
      [user.person_id].compact + Array(user.person&.patient_ids)
    end
  end
end
