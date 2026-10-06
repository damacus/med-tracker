# frozen_string_literal: true

class HouseholdPolicy < ApplicationPolicy
  def edit?
    household_manager? && household == record
  end

  def update?
    edit?
  end

  class Scope < ApplicationPolicy::Scope
    def resolve
      return scope.none unless household_manager?

      scope.where(id: household.id)
    end
  end
end
