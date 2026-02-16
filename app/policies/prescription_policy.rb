# frozen_string_literal: true

class PrescriptionPolicy < ApplicationPolicy
  def index?
    admin? || doctor? || nurse? || carer_with_patient? || parent_with_minor?
  end

  def show?
    can_take_own_medicine? || minor_with_own_prescription? || prescription_access?
  end

  def create?
    admin? || doctor?
  end

  alias new? create?

  def update?
    admin? || doctor?
  end

  alias edit? update?

  def destroy?
    admin? || doctor?
  end

  def take_medicine?
    can_take_own_medicine? || minor_with_supervision? || medical_staff_or_carer?
  end

  private

  def prescription_access?
    admin? || doctor? || nurse? || carer_with_patient? || parent_with_minor?
  end

  def minor_with_supervision?
    minor_with_own_prescription? && supervisor?
  end

  def supervisor?
    parent_with_minor? || carer_with_patient?
  end

  def medical_staff_or_carer?
    medical_staff? || supervisor?
  end

  def medical_staff?
    admin? || doctor? || nurse?
  end

  def can_take_own_medicine?
    return false unless user&.person

    user.person.id == record.person_id && user.person.adult?
  end

  def minor_with_own_prescription?
    return false unless user&.person

    user.person.id == record.person_id && user.person.minor?
  end

  def person_id_for_authorization
    record.person_id
  end

  class Scope < ApplicationPolicy::Scope
    def resolve
      return scope.none unless user
      return scope.all if admin_or_clinician?

      scope.where(person_id: accessible_person_ids)
    end
  end
end
