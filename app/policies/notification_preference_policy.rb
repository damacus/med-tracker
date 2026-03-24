# frozen_string_literal: true

class NotificationPreferencePolicy < ApplicationPolicy
  def show?
    return false unless user&.person

    admin_or_clinician? || record.person_id == user.person_id
  end

  class Scope < ApplicationPolicy::Scope
    def resolve
      return scope.none unless user
      return scope.all if admin_or_clinician?
      return scope.none unless user.person_id

      scope.where(person_id: user.person_id)
    end
  end
end
