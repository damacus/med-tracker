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

  def new?
    create?
  end

  def update?
    admin? || doctor?
  end

  def edit?
    update?
  end

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

  def carer_with_patient?
    caregiver_has_patient?(record_person_id)
  end

  def parent_with_minor?
    parent_has_minor_patient?(record_person_id)
  end

  class Scope < ApplicationPolicy::Scope
    def resolve
      return scope.none unless user
      return scope.all if full_access?
      return carer_parent_scope if care_relationships?
      return own_prescriptions_scope if owns_record?

      scope.none
    end

    private

    def full_access?
      admin? || doctor?
    end

    def care_relationships?
      care_relationship_patient_ids.any?
    end

    def carer_parent_scope
      scope.where(person_id: care_relationship_patient_ids)
    end

    def own_prescriptions_scope
      scope.where(person_id: user.person.id)
    end

    def owns_record?
      user&.person
    end
  end
end
