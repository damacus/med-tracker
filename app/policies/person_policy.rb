# frozen_string_literal: true

class PersonPolicy < ApplicationPolicy
  def index?
    admin? || medical_staff? || carer_or_parent?
  end

  def show?
    admin? || medical_staff? || owns_record? || carer_with_patient? || parent_with_minor?
  end

  def create?
    # SECURITY: Admins should not create people directly
    # People should only be created through proper user signup/invitation flows
    # to prevent abuse and ensure proper account/authentication setup
    # See: USER_MANAGEMENT_PLAN.md Phase 5.1 for invitation system
    false
  end

  alias new? create?

  def update?
    admin?
  end

  alias edit? update?

  def destroy?
    admin?
  end

  private

  def owns_record?
    user&.person == record
  end

  def person_id_for_authorization
    record.id
  end

  class Scope < ApplicationPolicy::Scope
    def resolve
      return scope.none unless user
      return scope.all if admin_or_clinician?
      return scope.none unless carer_or_parent? || user.parent?

      scope.where(id: accessible_person_ids)
    end

    private

    def accessible_person_ids
      ids = [user.person_id].compact
      ids.concat(Array(user.person&.patient_ids))
      ids.uniq
    end
  end
end
