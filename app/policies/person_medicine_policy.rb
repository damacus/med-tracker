# frozen_string_literal: true

class PersonMedicinePolicy < ApplicationPolicy
  def index?
    show?
  end

  def show?
    administrator_or_clinician? || carer_with_patient? || false
  end

  def create?
    user&.administrator? || false
  end

  def new?
    create?
  end

  def update?
    user&.administrator? || false
  end

  def edit?
    update?
  end

  def destroy?
    user&.administrator? || false
  end

  def take_medicine?
    user&.administrator? || carer_with_patient? || false
  end

  private

  def administrator_or_clinician?
    user&.administrator? || user&.doctor? || user&.nurse?
  end

  def carer_with_patient?
    (carer_or_parent? && user.person.patients.exists?(record.person.id)) || false
  end

  class Scope < ApplicationPolicy::Scope
    def resolve
      return scope.none unless user
      return scope.all if administrator_or_clinician?
      return scope.none unless carer_or_parent?

      scope.where(person_id: accessible_person_ids)
    end

    private

    def administrator_or_clinician?
      user&.administrator? || user&.doctor? || user&.nurse?
    end

    def carer_or_parent?
      user&.carer? || user&.parent?
    end

    def accessible_person_ids
      [user.person_id].compact + Array(user.person&.patient_ids)
    end
  end
end
