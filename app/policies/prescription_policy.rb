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

  alias_method :new?, :create?

  def update?
    admin? || doctor?
  end

  alias_method :edit?, :update?

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
    return false unless user&.carer? && user.person

    user.person.patients.exists?(record.person_id)
  end

  def parent_with_minor?
    return false unless user&.parent? && user.person

    user.person.patients.where(person_type: :minor).exists?(record.person_id)
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
      accessible_patient_ids.any?
    end

    def carer_parent_scope
      scope.where(person_id: accessible_patient_ids)
    end

    def own_prescriptions_scope
      scope.where(person_id: user.person.id)
    end

    def owns_record?
      user&.person
    end

    def carer_with_patient?
      user&.carer? && user.person
    end

    def parent_with_minor?
      user&.parent? && user.person
    end

    def accessible_patient_ids
      [].tap do |ids|
        ids.concat(carer_patient_ids) if user.carer?
        ids.concat(parent_minor_patient_ids) if user.parent?
      end
    end

    def carer_patient_ids
      Array(user.person&.patient_ids)
    end

    def parent_minor_patient_ids
      Array(Person.where(id: user.person&.patient_ids, person_type: :minor).pluck(:id))
    end
  end
end
