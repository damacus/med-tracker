# frozen_string_literal: true

class MedicinePolicy < ApplicationPolicy
  def index?
    admin? || doctor? || nurse?
  end

  def show?
    admin? || doctor? || nurse?
  end

  def create?
    admin? || doctor?
  end

  def new?
    create?
  end

  def update?
    admin? || doctor?
  end

  def edit?
    update?
  end

  def destroy?
    admin?
  end

  def finder?
    admin? || doctor? || nurse?
  end

  class Scope < ApplicationPolicy::Scope
    def resolve
      return scope.none unless user
      return scope.all if admin? || doctor? || nurse?

      scope.none
    end
  end
end
