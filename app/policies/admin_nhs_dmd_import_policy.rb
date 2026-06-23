# frozen_string_literal: true

class AdminNhsDmdImportPolicy < ApplicationPolicy
  def new?
    household_manager?
  end

  def create?
    household_manager?
  end
end
