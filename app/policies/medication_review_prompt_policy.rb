# frozen_string_literal: true

class MedicationReviewPromptPolicy < ApplicationPolicy
  def index?
    adult_household_member?
  end

  def show?
    review_prompt_access?(:view)
  end

  def update?
    review_prompt_access?(:manage)
  end

  private

  def adult_household_member?
    active_membership? && membership.person&.adult?
  end

  def review_prompt_access?(access_level)
    return false unless adult_household_member?
    return false unless same_household?(record)

    person_grant_allows?(record.person, access_level)
  end

  class Scope < ApplicationPolicy::Scope
    def resolve
      return scope.none unless active_membership? && membership.person&.adult?

      scope.where(household: household, person_id: granted_person_ids_for(:view))
    end
  end
end
