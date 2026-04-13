# frozen_string_literal: true

class SchedulesIndexQuery
  attr_reader :scope

  def initialize(scope:)
    @scope = scope
  end

  def call
    scope.active.includes(:person, :medication, :dosage).order(:start_date, :id)
  end
end
