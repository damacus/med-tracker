# frozen_string_literal: true

class MedicinePolicy < ApplicationPolicy
  def index?
    admin? || doctor? || nurse? || false
  end

  def show?
    admin? || doctor? || nurse? || false
  end

  def create?
    admin? || doctor? || false
  end

  def new?
    create?
  end

  def update?
    admin? || doctor? || false
  end

  def edit?
    update?
  end

  def destroy?
    admin? || false
  end

  def finder?
    admin? || doctor? || nurse? || false
  end

  private

  def admin?
    user&.administrator?
  end

  def doctor?
    user&.doctor?
  end

  def nurse?
    user&.nurse?
  end

  class Scope < ApplicationPolicy::Scope
    def resolve
      return scope.none unless user
      return scope.all if admin? || doctor? || nurse?

      scope.none
    end

    private

    def admin?
      user&.administrator?
    end

    def doctor?
      user&.doctor?
    end

    def nurse?
      user&.nurse?
    end
  end
end
