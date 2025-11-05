# frozen_string_literal: true

# Policy for AuditLog access
class AuditLogPolicy < ApplicationPolicy
  # Only administrators can view audit logs
  def index?
    user&.administrator?
  end

  # Nobody can create, update or destroy audit logs manually
  def create?
    false
  end

  def update?
    false
  end

  def destroy?
    false
  end

  # Scope audit logs to what the user can see
  class Scope < Scope
    def resolve
      if user&.administrator?
        scope.all
      else
        scope.none
      end
    end
  end
end
