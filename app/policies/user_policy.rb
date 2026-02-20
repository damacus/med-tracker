# frozen_string_literal: true

class UserPolicy < ApplicationPolicy
  def index?
    admin?
  end

  def show?
    admin? || user == record || false
  end

  def create?
    admin?
  end

  alias new? create?

  def update?
    admin? || user == record || false
  end

  alias edit? update?

  def destroy?
    admin?
  end

  def activate?
    admin?
  end

  class Scope < ApplicationPolicy::Scope
    def resolve
      return scope.none unless user
      return scope.all if admin?

      scope.where(id: user.id)
    end
  end
end
