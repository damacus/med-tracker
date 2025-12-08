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
  end
end
