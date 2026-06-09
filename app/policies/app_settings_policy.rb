# frozen_string_literal: true

class AppSettingsPolicy < ApplicationPolicy
  def show?
    admin?
  end

  def update?
    admin?
  end

  class Scope < ApplicationPolicy::Scope
    def resolve
      scope
    end
  end
end
