# frozen_string_literal: true

class NativeDeviceTokenPolicy < ApplicationPolicy
  def create?
    account_owned?
  end

  def destroy?
    account_owned?
  end

  private

  def account_owned?
    actor_account.present? && record.account_id == actor_account.id
  end

  def actor_account
    account || user&.person&.account
  end
end
