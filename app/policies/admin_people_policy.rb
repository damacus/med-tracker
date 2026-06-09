# frozen_string_literal: true

class AdminPeoplePolicy < ApplicationPolicy
  def index?
    user&.administrator? || false
  end

  class Scope < ApplicationPolicy::Scope
    def resolve
      scope
    end
  end
end
