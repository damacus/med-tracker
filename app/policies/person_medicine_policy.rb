# frozen_string_literal: true

class PersonMedicinePolicy < ApplicationPolicy
  def index?
    show?
  end

  def show?
    admin_or_clinician? || self_or_dependent? || carer_with_patient?
  end

  def create?
    admin? || self_or_dependent?
  end

  alias new? create?

  def update?
    admin? || self_or_dependent?
  end

  alias edit? update?

  def destroy?
    admin? || self_or_dependent?
  end

  def take_medicine?
    admin? || self_or_dependent? || carer_with_patient?
  end

  private

  def self_or_dependent?
    return false unless user&.person

    person = record.is_a?(Class) ? nil : record.person
    return true if person.nil? # For new? action with PersonMedicine class - actual authorization happens in create?
    return true if person.id == user.person_id # Self

    false
  end

  def person_id_for_authorization
    record.respond_to?(:person) && record.person ? record.person.id : nil
  end

  class Scope < ApplicationPolicy::Scope
    def resolve
      return scope.none unless user
      return scope.all if admin_or_clinician?

      # Users can see their own person medicines and those of their dependents
      scope.where(person_id: accessible_person_ids)
    end
  end
end
