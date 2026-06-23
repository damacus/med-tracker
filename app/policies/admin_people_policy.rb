# frozen_string_literal: true

class AdminPeoplePolicy < ApplicationPolicy
  def index?
    household_manager?
  end

  class Scope < ApplicationPolicy::Scope
    def resolve
      scope
    end
  end
end
