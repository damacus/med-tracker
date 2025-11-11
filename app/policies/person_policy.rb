# frozen_string_literal: true

class PersonPolicy < ApplicationPolicy
  def index?
    admin? || medical_staff? || carer_or_parent?
  end

  def show?
    admin? || medical_staff? || owns_record? || carer_with_patient? || parent_with_minor?
  end

  def create?
    admin?
  end

  def update?
    admin?
  end

  def destroy?
    admin?
  end

  private

  def owns_record?
    user&.person == record
  end

  def carer_with_patient?
    return false unless carer_or_parent? && user&.person

    user.person.patients.exists?(record.id)
  end

  def parent_with_minor?
    return false unless user&.parent? && user.person

    user.person.patients.where(person_type: :minor).exists?(record.id)
  end

  class Scope < ApplicationPolicy::Scope
    def resolve
      return scope.none unless user
      return scope.all if admin_or_clinician?
      return scope.none unless carer_or_parent? || user.parent?

      scope.where(id: accessible_person_ids)
    end

    private

    def accessible_person_ids
      ids = [user.person_id].compact
      ids.concat(Array(user.person&.patient_ids))
      ids.uniq
    end
  end
end
