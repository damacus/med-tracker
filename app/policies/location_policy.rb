# frozen_string_literal: true

class LocationPolicy < ApplicationPolicy
  def index?
    user.present?
  end

  def show?
    user.present?
  end

  def create?
    admin?
  end

  alias new? create?

  def update?
    admin?
  end

  alias edit? update?

  def destroy?
    admin?
  end

  class Scope < ApplicationPolicy::Scope
    def resolve
      return scope.none unless user

      scope.all
    end
  end
end
