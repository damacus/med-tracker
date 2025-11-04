# frozen_string_literal: true

class DashboardPolicy < ApplicationPolicy
  def index?
    user.present?
  end

  class Scope < ApplicationPolicy::Scope
    def resolve
      scope
    end
  end
end
