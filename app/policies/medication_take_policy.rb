# frozen_string_literal: true

class MedicationTakePolicy < ApplicationPolicy
  def create?
    admin? || medical_staff? || carer_with_patient? || parent_with_minor? || adult_with_own_prescription?
  end

  def new?
    create?
  end

  private

  def adult_with_own_prescription?
    return false unless user&.person

    user.person.id == record.prescription.person_id && user.person.adult?
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
      return own_takes_scope if owns_record?

      scope.none
    end

    private

    def full_access?
      admin? || medical_staff?
    end

    def care_relationships?
      care_relationship_patient_ids.any?
    end

    def carer_parent_scope
      scope.joins(:prescription).where(prescriptions: { person_id: care_relationship_patient_ids })
    end

    def own_takes_scope
      scope.joins(:prescription).where(prescriptions: { person_id: user.person.id })
    end

    def owns_record?
      user&.person
    end
  end
end
