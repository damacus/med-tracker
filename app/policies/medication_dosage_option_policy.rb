# frozen_string_literal: true

class MedicationDosageOptionPolicy < DosagePolicy
  def index?
    active_membership?
  end

  class Scope < ApplicationPolicy::Scope
    def resolve
      return scope.none unless active_membership?

      scope.where(household: household)
    end
  end
end
