# frozen_string_literal: true

class PeopleIndexQuery
  attr_reader :scope

  def initialize(scope:)
    @scope = scope
  end

  def call
    scope.includes(:user, :schedules, :person_medications)
  end
end
