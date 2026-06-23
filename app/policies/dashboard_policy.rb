# frozen_string_literal: true

class DashboardPolicy < ApplicationPolicy
  def index?
    active_membership?
  end

  class Scope < ApplicationPolicy::Scope
    def resolve
      scope
    end
  end
end
