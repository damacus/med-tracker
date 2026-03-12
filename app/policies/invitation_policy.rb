# frozen_string_literal: true

class InvitationPolicy < ApplicationPolicy
  def index?
    admin?
  end

  def create?
    admin?
  end

  def resend?
    admin?
  end
end
