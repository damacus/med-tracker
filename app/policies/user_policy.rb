# frozen_string_literal: true

class UserPolicy < ApplicationPolicy
  def index?
    user&.administrator? || false
  end

  def show?
    user&.administrator? || user == record || false
  end

  def create?
    user&.administrator? || false
  end

  def update?
    user&.administrator? || user == record || false
  end

  def destroy?
    user&.administrator? || false
  end

  def activate?
    user&.administrator? || false
  end

  class Scope < ApplicationPolicy::Scope
    def resolve
      return scope.none unless user
      return scope.all if user.administrator?

      scope.where(id: user.id)
    end
  end
end
