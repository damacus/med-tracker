# frozen_string_literal: true

class PlatformUserPolicy < ApplicationPolicy
  def update?
    platform_admin?
  end
end
