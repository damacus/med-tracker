# frozen_string_literal: true

class UserPolicy < ApplicationPolicy
  def index?
    household_manager?
  end

  def show?
    household_user_admin?
  end

  def create?
    household_manager?
  end

  alias new? create?

  def update?
    household_user_admin?
  end

  alias edit? update?

  def destroy?
    household_user_admin?
  end

  def activate?
    household_user_admin?
  end

  def verify?
    household_user_admin?
  end

  class Scope < ApplicationPolicy::Scope
    def resolve
      return scope.none unless household_manager?

      person_scope = scope.joins(:person)
      person_scope.where(people: { account_id: household_account_ids }).or(
        person_scope.where(people: { household_id: household.id })
      )
    end

    private

    def household_account_ids
      household.household_memberships.active.select(:account_id)
    end
  end

  private

  def household_user_admin?
    household_manager? && household_user_record?
  end

  def household_user_record?
    return true if record.is_a?(Class)
    return true if record.person&.household_id == household.id

    account_id = record.person&.account_id
    account_id.present? && household.household_memberships.active.exists?(account_id: account_id)
  end
end
