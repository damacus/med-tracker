# frozen_string_literal: true

class CarerRelationshipPolicy < ApplicationPolicy
  def index?
    admin_or_clinician?
  end

  def show?
    admin_or_clinician? || carer_owns_relationship?
  end

  def create?
    admin?
  end

  def update?
    admin?
  end

  def destroy?
    admin?
  end

  def activate?
    admin?
  end

  private

  def carer_owns_relationship?
    (user&.person && record.carer_id == user.person.id) || false
  end

  class Scope < ApplicationPolicy::Scope
    def resolve
      return scope.none unless user
      return scope.all if admin_or_clinician?

      scope.where(carer_id: user.person_id)
    end
  end
end
