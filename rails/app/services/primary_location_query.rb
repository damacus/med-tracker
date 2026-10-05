# frozen_string_literal: true

class PrimaryLocationQuery
  attr_reader :person

  def initialize(person:)
    @person = person
  end

  def call
    return nil unless person

    person.location_memberships.order(:id).first&.location
  end
end
