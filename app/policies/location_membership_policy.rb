# frozen_string_literal: true

class LocationMembershipPolicy < ApplicationPolicy
  def create?
    household_manager?
  end

  def destroy?
    household_manager? && same_household?(record)
  end
end
