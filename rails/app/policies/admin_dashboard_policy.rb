# frozen_string_literal: true

class AdminDashboardPolicy < ApplicationPolicy
  def index?
    household_manager?
  end

  class Scope < ApplicationPolicy::Scope
    def resolve
      scope
    end
  end
end
