# frozen_string_literal: true

# Policy for audit log access
# Only administrators can view audit logs (read-only)
class AuditLogPolicy < ApplicationPolicy
  def index?
    user.administrator?
  end

  def show?
    user.administrator?
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
