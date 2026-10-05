# frozen_string_literal: true

class ManagedNotificationGrantsQuery
  def initialize(membership:)
    @membership = membership
  end

  def call
    return [] unless membership&.active?

    grants.sort_by { |grant| [grant.person.name.downcase, grant.person_id] }
  end

  private

  attr_reader :membership

  def grants
    scope = membership.person_access_grants.active.manage.includes(:person)
    scope = scope.where.not(person_id: membership.person_id) if membership.person_id
    scope.to_a
  end
end
