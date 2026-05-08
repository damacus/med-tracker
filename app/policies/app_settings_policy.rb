# frozen_string_literal: true

class AppSettingsPolicy < ApplicationPolicy
  def show?
    user&.administrator?
  end

  def update?
    user&.administrator?
  end

  class Scope < ApplicationPolicy::Scope
    def resolve
      scope
    end
  end
end
