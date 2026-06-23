# frozen_string_literal: true

class MedicationTakePolicy < ApplicationPolicy
  def create?
    person_grant_allows?(record.person, :record)
  end

  def new?
    create?
  end

  private

  def person_id_for_authorization
    record.person&.id
  end

  class Scope < ApplicationPolicy::Scope
    def resolve
      household_medication_take_scope
    end

    private

    def household_medication_take_scope
      return scope.none unless active_membership?

      household_scope = scope.where(household: household)
      return household_scope if household_manager?

      joined_scope = household_scope.left_joins(:schedule, :person_medication)
      granted_person_ids = granted_person_ids_for(:view)

      joined_scope
        .where(schedules: { person_id: granted_person_ids })
        .or(joined_scope.where(person_medications: { person_id: granted_person_ids }))
    end
  end
end
