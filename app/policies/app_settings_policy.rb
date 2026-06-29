# frozen_string_literal: true

class AppSettingsPolicy < ApplicationPolicy
  def show?
    platform_admin?
  end

  def update?
    platform_admin?
  end

  class Scope < ApplicationPolicy::Scope
    def resolve
      scope
    end
  end
end
