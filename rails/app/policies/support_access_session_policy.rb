# frozen_string_literal: true

class SupportAccessSessionPolicy < ApplicationPolicy
  def create?
    platform_admin? && record.respond_to?(:household) && record.household&.operational?
  end

  def destroy?
    platform_admin?
  end

  class Scope < ApplicationPolicy::Scope
    def resolve
      return scope.none unless account&.platform_admin&.active?

      scope.where(platform_admin: account.platform_admin)
    end
  end
end
