# frozen_string_literal: true

class ApplicationPolicy
  include PolicyHelpers

  attr_reader :context, :user, :record

  def initialize(user, record)
    @context = user if user.is_a?(AuthorizationContext)
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
    false
  end

  def parent_with_dependent_patient?
    false
  end

  class Scope
    include PolicyHelpers

    def initialize(user, scope)
      @context = user if user.is_a?(AuthorizationContext)
      @user = user
      @scope = scope
    end

    def resolve
      raise NoMethodError, "You must define #resolve in #{self.class}"
    end

    private

    attr_reader :context, :user, :scope

    def accessible_person_ids = []
  end

  def active_patient_relationships
    CarerRelationship.none
  end
end
