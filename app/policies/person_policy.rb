# frozen_string_literal: true

class PersonPolicy < ApplicationPolicy
  def index?
    admin? || clinician? || carer_role? || false
  end

  def show?
    admin? || clinician? || owns_record? || carer_with_patient? || parent_with_minor? || false
  end

  def create?
    admin? || false
  end

  def update?
    admin? || false
  end

  def destroy?
    admin? || false
  end

  private

  def admin?
    user&.administrator?
  end

  def clinician?
    user&.doctor? || user&.nurse?
  end

  def carer_role?
    user&.carer? || user&.parent?
  end

  def owns_record?
    user&.person == record
  end

  def carer_with_patient?
    return false unless carer_role? && user&.person

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
      return scope.none unless carer_role? || user.parent?

      scope.where(id: accessible_person_ids)
    end

    private

    def admin_or_clinician?
      user&.administrator? || user&.doctor? || user&.nurse?
    end

    def carer_role?
      user&.carer? || user&.parent?
    end

    def accessible_person_ids
      ids = [user.person_id].compact
      ids.concat(Array(user.person&.patient_ids))
      ids.uniq
    end
  end
end
