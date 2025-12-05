# frozen_string_literal: true

class ApplicationPolicy
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

  def new?
    create?
  end

  def update?
    false
  end

  def edit?
    update?
  end

  def destroy?
    false
  end

  private

  def record_person_id
    return if record.nil? || record.is_a?(Class)
    return record.id if record.is_a?(Person)

    return record.prescription&.person_id if record.respond_to?(:prescription)

    if record.respond_to?(:person)
      person = record.person
      return person.id if person
    end

    record.person_id if record.respond_to?(:person_id)
  end

  def patient_for_carer?(person_id)
    return false unless user&.carer? && user.person && person_id

    user.person.patients.exists?(id: person_id)
  end

  def patient_for_parent?(person_id, minors_only: false)
    return false unless user&.parent? && user.person && person_id

    relation = user.person.patients
    relation = relation.where(person_type: :minor) if minors_only
    relation.exists?(id: person_id)
  end

  def caregiver_has_patient?(person_id)
    patient_for_carer?(person_id) || patient_for_parent?(person_id)
  end

  def parent_has_minor_patient?(person_id)
    patient_for_parent?(person_id, minors_only: true)
  end

  def admin?
    user&.administrator? || false
  end

  def admin_or_clinician?
    user&.administrator? || user&.doctor? || user&.nurse? || false
  end

  def doctor?
    user&.doctor? || false
  end

  def nurse?
    user&.nurse? || false
  end

  def medical_staff?
    user&.doctor? || user&.nurse? || false
  end

  def carer_or_parent?
    user&.carer? || user&.parent? || false
  end

  class Scope
    def initialize(user, scope)
      @user = user
      @scope = scope
    end

    def resolve
      raise NoMethodError, "You must define #resolve in #{self.class}"
    end

    private

    attr_reader :user, :scope

    def admin?
      user&.administrator? || false
    end

    def admin_or_clinician?
      user&.administrator? || user&.doctor? || user&.nurse? || false
    end

    def doctor?
      user&.doctor? || false
    end

    def nurse?
      user&.nurse? || false
    end

    def medical_staff?
      user&.doctor? || user&.nurse? || false
    end

    def carer_or_parent?
      user&.carer? || user&.parent? || false
    end

    def carer_patient_ids
      return [] unless user&.carer? && user.person

      Array(user.person.patient_ids)
    end

    def parent_minor_patient_ids
      return [] unless user&.parent? && user.person

      Person.where(id: user.person.patient_ids, person_type: :minor).pluck(:id)
    end

    def care_relationship_patient_ids
      (carer_patient_ids + parent_minor_patient_ids).uniq
    end
  end
end
