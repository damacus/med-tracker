# frozen_string_literal: true

class InvitationPolicy < ApplicationPolicy
  def index?
    household_manager?
  end

  def create?
    household_manager?
  end

  def resend?
    household_manager?
  end

  def destroy?
    household_manager?
  end
end
