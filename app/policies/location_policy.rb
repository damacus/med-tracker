# frozen_string_literal: true

class LocationPolicy < ApplicationPolicy
  def index?
    admin? || doctor? || nurse?
  end

  def show?
    admin? || doctor? || nurse?
  end

  def create?
    admin? || doctor?
  end

  alias new? create?

  def update?
    admin? || doctor?
  end

  alias edit? update?

  def destroy?
    admin?
  end

  class Scope < ApplicationPolicy::Scope
    def resolve
      return scope.none unless user
      return scope.all if admin? || doctor? || nurse?

      scope.none
    end
  end
end
