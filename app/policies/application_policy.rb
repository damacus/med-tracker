# frozen_string_literal: true

class ApplicationPolicy
  include PolicyHelpers

  attr_reader :user, :record

  def initialize(user, record)
    @user = user
    @record = record
  end

  def index?
    false
  end

  def show?
    false
  end

  def create?
    false
  end

  alias new? create?

  def update?
    false
  end

  alias edit? update?

  def destroy?
    false
  end

  private

  def person_id_for_authorization
    raise NotImplementedError, 'Subclass must implement #person_id_for_authorization'
  end

  def carer_with_patient?
    return false unless user&.carer? && user.person

    CarerRelationship.active.exists?(carer_id: user.person_id, patient_id: person_id_for_authorization)
  end

  def parent_with_dependent_patient?
    return false unless user&.parent? && user.person

    active_patient_relationships
      .joins(:patient)
      .where(patient_id: person_id_for_authorization)
      .exists?(people: { person_type: %i[minor dependent_adult], has_capacity: false })
  end

  class Scope
    include PolicyHelpers

    def initialize(user, scope)
      @user = user
      @scope = scope
    end

    def resolve
      raise NoMethodError, "You must define #resolve in #{self.class}"
    end

    private

    attr_reader :user, :scope

    def accessible_patient_ids
      [].tap do |ids|
        ids.concat(carer_patient_ids) if user.carer?
        ids.concat(parent_dependent_patient_ids) if user.parent?
      end
    end

    def carer_patient_ids
      active_patient_relationships.pluck(:patient_id)
    end

    def parent_dependent_patient_ids
      Person.where(
        id: active_patient_relationships.select(:patient_id),
        person_type: %i[minor dependent_adult],
        has_capacity: false
      ).pluck(:id)
    end

    def accessible_person_ids
      ids = Set.new([user.person_id].compact)
      ids.merge(accessible_patient_ids)
      ids.to_a
    end

    def active_patient_relationships
      return CarerRelationship.none unless user&.person_id

      CarerRelationship.active.where(carer_id: user.person_id)
    end
  end

  def active_patient_relationships
    return CarerRelationship.none unless user&.person_id

    CarerRelationship.active.where(carer_id: user.person_id)
  end
end
