# frozen_string_literal: true

class LocationsQuery
  attr_reader :scope

  def initialize(scope:)
    @scope = scope
  end

  def index
    scoped_locations
  end

  def find(id:)
    scoped_locations.find(id)
  end

  def options
    scope.order(:name)
  end

  private

  def scoped_locations
    scope.includes(:medications, :members, :location_memberships)
  end
end
