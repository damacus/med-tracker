# frozen_string_literal: true

class AdminNhsDmdImportPolicy < ApplicationPolicy
  def new?
    user&.administrator?
  end

  def create?
    user&.administrator?
  end
end
