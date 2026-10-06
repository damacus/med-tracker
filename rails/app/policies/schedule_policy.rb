# frozen_string_literal: true

class SchedulePolicy < ApplicationPolicy
  def index?
    household_schedule_index_access?
  end

  def show?
    person_grant_allows?(record.person, :view)
  end

  def create?
    schedule_person_grant_allows?(:manage)
  end

  alias new? create?

  def update?
    schedule_person_grant_allows?(:manage)
  end

  alias edit? update?

  def destroy?
    schedule_person_grant_allows?(:manage)
  end

  def take_medication?
    schedule_person_grant_allows?(:record)
  end

  private

  def household_schedule_index_access?
    active_membership? && membership.person&.adult?
  end

  def person_id_for_authorization
    record.person_id
  end

  def schedule_person_grant_allows?(access_level)
    return household_manager? if record.is_a?(Class)

    person_grant_allows?(record.person, access_level)
  end

  class Scope < ApplicationPolicy::Scope
    def resolve
      household_schedule_scope
    end

    private

    def household_schedule_scope
      return scope.none unless active_membership?

      scope.current.where(household: household, person_id: granted_person_ids_for(:view))
    end
  end
end
