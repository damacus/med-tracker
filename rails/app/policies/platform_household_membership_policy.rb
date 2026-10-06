# frozen_string_literal: true

class PlatformHouseholdMembershipPolicy < ApplicationPolicy
  def promote_owner?
    platform_admin?
  end
end
