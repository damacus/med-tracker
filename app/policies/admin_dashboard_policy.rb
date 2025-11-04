# frozen_string_literal: true

class AdminDashboardPolicy < ApplicationPolicy
  def index?
    user&.administrator?
  end

  class Scope < ApplicationPolicy::Scope
    def resolve
      scope
    end
  end
end
