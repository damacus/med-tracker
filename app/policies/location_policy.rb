# frozen_string_literal: true

class LocationPolicy < ApplicationPolicy
  def index?
    user.present?
  end

  def show?
    return false unless user

    admin_or_clinician? || accessible_location_ids.include?(record.id)
  end

  def create?
    admin?
  end

  alias new? create?

  def update?
    admin?
  end

  alias edit? update?

  def destroy?
    admin?
  end

  class Scope < ApplicationPolicy::Scope
    def resolve
      return scope.none unless user
      return scope.all if admin_or_clinician?

      location_ids = accessible_location_ids
      return scope.none if location_ids.empty?

      scope.where(id: location_ids)
    end

    private

    def accessible_location_ids
      person_ids = accessible_person_ids
      return [] if person_ids.empty?

      LocationMembership.where(person_id: person_ids).select(:location_id).distinct.pluck(:location_id)
    end

    def accessible_person_ids
      return [] unless user&.person

      [user.person_id, *carer_patient_ids, *parent_dependent_patient_ids].compact.uniq
    end

    def carer_patient_ids
      user.carer? ? active_patient_relationships.pluck(:patient_id) : []
    end

    def parent_dependent_patient_ids
      return [] unless user.parent?

      Person.where(
        id: active_patient_relationships.select(:patient_id),
        person_type: %i[minor dependent_adult],
        has_capacity: false
      ).pluck(:id)
    end
  end

  private

  def accessible_location_ids
    @accessible_location_ids ||= begin
      ids = accessible_person_ids_for_policy
      if ids.empty?
        []
      else
        LocationMembership.where(person_id: ids).pluck(:location_id).uniq
      end
    end
  end

  def accessible_person_ids_for_policy
    return [] unless user&.person

    [user.person_id, *carer_patient_ids_for_policy, *parent_dependent_patient_ids_for_policy].compact.uniq
  end

  def carer_patient_ids_for_policy
    user.carer? ? active_patient_relationships.pluck(:patient_id) : []
  end

  def parent_dependent_patient_ids_for_policy
    return [] unless user.parent?

    Person.where(
      id: active_patient_relationships.select(:patient_id),
      person_type: %i[minor dependent_adult],
      has_capacity: false
    ).pluck(:id)
  end
end
