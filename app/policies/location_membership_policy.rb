# frozen_string_literal: true

class LocationMembershipPolicy < ApplicationPolicy
  def create?
    admin?
  end

  def destroy?
    admin?
  end
end
