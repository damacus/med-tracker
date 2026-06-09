# frozen_string_literal: true

class AdminNhsDmdImportPolicy < ApplicationPolicy
  def new?
    admin?
  end

  def create?
    admin?
  end
end
