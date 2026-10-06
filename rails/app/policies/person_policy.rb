# frozen_string_literal: true

class PersonPolicy < ApplicationPolicy
  def index?
    active_membership?
  end

  def show?
    person_grant_allows?(record, :view)
  end

  def new?
    household_manager? || any_person_grant_allows?(:manage)
  end

  def create?
    return false unless household_manager? || any_person_grant_allows?(:manage)

    new_record_belongs_to_current_household?
  end

  def update?
    person_grant_allows?(record, :manage)
  end

  alias edit? update?

  def destroy?
    person_grant_allows?(record, :manage)
  end

  def add_medication?
    person_grant_allows?(record, :manage)
  end

  def download_health_history?
    person_grant_allows?(record, :manage)
  end

  private

  def new_record_belongs_to_current_household?
    record.household_id.blank? || record.household_id == household.id
  end

  def person_id_for_authorization
    record.id
  end

  class Scope < ApplicationPolicy::Scope
    def resolve
      resolve_for(:view)
    end

    def resolve_for(access_level)
      return scope.none unless active_membership?

      scope.where(
        household: household,
        id: granted_person_ids_for(access_level)
      )
    end
  end
end
