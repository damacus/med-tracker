# frozen_string_literal: true

class HealthEventPolicy < ApplicationPolicy
  def index?
    active_membership?
  end

  def show?
    health_event_person_grant_allows?(:view)
  end

  def create?
    health_event_person_grant_allows?(:record)
  end

  alias new? create?

  def update?
    health_event_person_grant_allows?(:manage)
  end

  alias edit? update?

  def destroy?
    health_event_person_grant_allows?(:manage)
  end

  private

  def person_id_for_authorization
    record.respond_to?(:person) && record.person ? record.person.id : nil
  end

  def health_event_person_grant_allows?(access_level)
    return household_manager? if record.is_a?(Class)

    person_grant_allows?(record.person, access_level)
  end

  class Scope < ApplicationPolicy::Scope
    def resolve
      household_health_event_scope
    end

    private

    def household_health_event_scope
      return scope.none unless active_membership?

      household_scope = scope.where(household: household)
      return household_scope if household_manager?

      household_scope.where(person_id: granted_person_ids_for(:view))
    end
  end
end
