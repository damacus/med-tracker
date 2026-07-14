# frozen_string_literal: true

class PersonAccessGrantPolicy < ApplicationPolicy
  def index?
    return household_owner? || household_administrator? if class_record?

    same_household?(record) && (household_owner? || household_administrator?)
  end

  private

  def class_record?
    record.is_a?(Class)
  end

  class Scope < ApplicationPolicy::Scope
    def resolve
      return scope.none unless active_membership? && (household_owner? || household_administrator?)

      scope.where(household: household)
    end

    private

    def household_owner?
      membership.owner?
    end

    def household_administrator?
      membership.administrator?
    end
  end
end
