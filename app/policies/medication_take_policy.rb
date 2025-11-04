# frozen_string_literal: true

class MedicationTakePolicy < ApplicationPolicy
  def create?
    admin? || medical_staff? || carer_with_patient? || parent_with_minor? || adult_with_own_prescription?
  end

  def new?
    create?
  end

  private

  def admin?
    user&.administrator?
  end

  def medical_staff?
    user&.doctor? || user&.nurse?
  end

  def adult_with_own_prescription?
    return false unless user&.person

    user.person.id == record.prescription.person_id && user.person.adult?
  end

  def carer_with_patient?
    return false unless user&.carer? && user.person

    user.person.patients.exists?(record.prescription.person_id)
  end

  def parent_with_minor?
    return false unless user&.parent? && user.person

    user.person.patients.where(person_type: :minor).exists?(record.prescription.person_id)
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
      accessible_patient_ids.any?
    end

    def carer_parent_scope
      scope.joins(:prescription).where(prescriptions: { person_id: accessible_patient_ids })
    end

    def own_takes_scope
      scope.joins(:prescription).where(prescriptions: { person_id: user.person.id })
    end

    def admin?
      user&.administrator?
    end

    def medical_staff?
      user&.doctor? || user&.nurse?
    end

    def owns_record?
      user&.person
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
