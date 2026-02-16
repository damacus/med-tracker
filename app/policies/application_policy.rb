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

  alias new? create?

  def update?
    false
  end

  alias edit? update?

  def destroy?
    false
  end

  private

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

  def person_id_for_authorization
    raise NotImplementedError, 'Subclass must implement #person_id_for_authorization'
  end

  def carer_with_patient?
    return false unless carer_or_parent? && user&.person

    user.person.patients.exists?(person_id_for_authorization)
  end

  def parent_with_minor?
    return false unless user&.parent? && user.person

    user.person.patients.where(person_type: :minor).exists?(person_id_for_authorization)
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

    def accessible_patient_ids
      [].tap do |ids|
        ids.concat(carer_patient_ids) if user.carer?
        ids.concat(parent_minor_patient_ids) if user.parent?
      end
    end

    def carer_patient_ids
      Array(user.person&.patient_ids)
    end

    def parent_minor_patient_ids
      Array(Person.where(id: user.person&.patient_ids, person_type: :minor).pluck(:id))
    end

    def accessible_person_ids
      ids = Set.new([user.person_id].compact)
      ids.merge(accessible_patient_ids)
      ids.to_a
    end
  end
end
