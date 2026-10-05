# frozen_string_literal: true

class AdminNhsDmdImportPolicy < ApplicationPolicy
  def new?
    platform_admin?
  end

  def create?
    platform_admin?
  end
end
