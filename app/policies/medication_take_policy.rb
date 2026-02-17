# frozen_string_literal: true

class MedicationTakePolicy < ApplicationPolicy
  def create?
    admin? || medical_staff? || carer_with_patient? || parent_with_minor? || adult_with_own_medicine?
  end

  def new?
    create?
  end

  private

  def adult_with_own_medicine?
    return false unless user&.person

    user.person.id == record.person&.id && user.person.adult?
  end

  def person_id_for_authorization
    record.person&.id
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
      ids = accessible_patient_ids
      scope.left_joins(:prescription, :person_medicine)
           .where('prescriptions.person_id IN (:ids) OR person_medicines.person_id IN (:ids)', ids: ids)
    end

    def own_takes_scope
      person_id = user.person.id
      scope.left_joins(:prescription, :person_medicine)
           .where('prescriptions.person_id = :id OR person_medicines.person_id = :id', id: person_id)
    end

    def owns_record?
      user&.person
    end
  end
end
