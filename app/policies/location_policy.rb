# frozen_string_literal: true

class LocationPolicy < ApplicationPolicy
  def index?
    active_membership?
  end

  def show?
    active_membership? && same_household?(record)
  end

  def create?
    household_manager?
  end

  alias new? create?

  def update?
    household_manager? && same_household?(record)
  end

  alias edit? update?

  def destroy?
    household_manager? && same_household?(record)
  end

  class Scope < ApplicationPolicy::Scope
    def resolve
      active_membership? ? scope.where(household: household) : scope.none
    end
  end
end
