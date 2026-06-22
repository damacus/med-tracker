# frozen_string_literal: true

class DependentAccessRequestPolicy < ApplicationPolicy
  def index?
    admin?
  end

  def create?
    parent_requester? && requester_matches_record? && dependent_patient?
  end

  def approve?
    admin? && record.pending? && record.requester != user
  end

  def reject?
    admin? && record.pending?
  end

  class Scope < ApplicationPolicy::Scope
    def resolve
      return scope.none unless user
      return scope.all if admin?

      scope.where(requester_id: user.id)
    end
  end

  private

  def parent_requester?
    user&.parent? && user.person&.person_type == 'adult' && user.person.has_capacity?
  end

  def requester_matches_record?
    record.requester == user && record.carer_id == user.person_id
  end

  def dependent_patient?
    record.patient&.person_type.in?(%w[minor dependent_adult]) && record.patient.has_capacity == false
  end
end
