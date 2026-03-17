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
    return true if admin? || doctor?
    return false unless user&.parent?
    return true if record.is_a?(Class) || record.location_id.blank?

    authorized_location_scope.exists?(id: record.location_id)
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

  alias finder? create?

  class Scope < ApplicationPolicy::Scope
    def resolve
      return scope.none unless user
      return scope.all if admin? || doctor? || nurse?
      return scope.none unless carer_or_parent?

      scope_by_location
    end

    private

    def scope_by_location
      scope.where(location_id: authorized_location_scope.select(:id))
    end

    def authorized_location_scope
      LocationPolicy::Scope.new(user, Location.all).resolve
    end
  end

  private

  def authorized_location_scope
    LocationPolicy::Scope.new(user, Location.all).resolve
  end
end
