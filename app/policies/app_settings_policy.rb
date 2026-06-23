# frozen_string_literal: true

class AppSettingsPolicy < ApplicationPolicy
  def show?
    household_manager?
  end

  def update?
    household_manager?
  end

  class Scope < ApplicationPolicy::Scope
    def resolve
      scope
    end
  end
end
