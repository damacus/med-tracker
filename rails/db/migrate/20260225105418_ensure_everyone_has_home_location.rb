# frozen_string_literal: true

class EnsureEveryoneHasHomeLocation < ActiveRecord::Migration[8.1]
  def up
    location = Location.find_or_create_by!(name: 'Home') do |loc|
      loc.description = 'Primary home location'
    end

    # Find people who don't have any location membership
    people_without_location = Person.where.not(id: LocationMembership.select(:person_id))

    people_without_location.each do |person|
      LocationMembership.find_or_create_by!(location: location, person: person)
    end
  end

  def down
    # No-op: we don't want to remove memberships that might have been intentional
  end
end
