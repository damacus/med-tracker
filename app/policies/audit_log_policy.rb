# frozen_string_literal: true

# Policy for audit log access
# Only administrators can view audit logs (read-only)
class AuditLogPolicy < ApplicationPolicy
  class Scope < ApplicationPolicy::Scope
    def resolve
      return scope.none unless household_manager? && context&.household

      scope.where(household_id: context.household.id)
    end
  end

  def index?
    household_manager?
  end

  def show?
    household_manager?
  end

  # Audit logs are read-only - no create, update, or destroy
  def create?
    false
  end

  def update?
    false
  end

  def destroy?
    false
  end
end
