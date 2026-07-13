# frozen_string_literal: true

class HouseholdRetentionHoldPolicy < ApplicationPolicy
  def create?
    platform_admin?
  end

  def update?
    platform_admin?
  end
end
