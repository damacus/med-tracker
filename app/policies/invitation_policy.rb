# frozen_string_literal: true

class InvitationPolicy < ApplicationPolicy
  def index?
    admin?
  end

  def create?
    admin?
  end
end
