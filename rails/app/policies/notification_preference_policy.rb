# frozen_string_literal: true

class NotificationPreferencePolicy < ApplicationPolicy
  def show?
    person_grant_allows?(record.person, :view)
  end

  def update?
    person_grant_allows?(record.person, :manage)
  end

  class Scope < ApplicationPolicy::Scope
    def resolve
      household_notification_preference_scope
    end

    private

    def household_notification_preference_scope
      return scope.none unless active_membership?

      scope.where(household: household, person_id: granted_person_ids_for(:view))
    end
  end
end
