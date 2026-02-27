# frozen_string_literal: true

class MedicationPolicy < ApplicationPolicy
  def index?
    admin? || doctor? || nurse? || carer_or_parent?
  end

  def show?
    admin? || doctor? || nurse? || carer_or_parent?
  end

  alias dosages? show?

  def create?
    admin? || doctor? || user&.parent? || false
  end

  alias new? create?

  def update?
    admin? || doctor?
  end

  alias edit? update?

  def refill?
    update? || nurse? || carer_or_parent?
  end

  def mark_as_ordered?
    refill?
  end

  def mark_as_received?
    refill?
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
      return scope.none unless carer_or_parent?

      scope_by_location
    end

    private

    def scope_by_location
      scope.where(location_id: user.person&.location_ids || [])
    end
  end
end
