# frozen_string_literal: true

class EnsureEveryoneHasHomeLocation < ActiveRecord::Migration[8.1]
  def up
    # Find people who don't have any location membership and give each one an
    # isolated default location. Sharing a single global "Home" location would
    # collapse the location boundary used by inventory authorization.
    people_without_location = Person.where.not(id: LocationMembership.select(:person_id))

    people_without_location.find_each do |person|
      location = Location.create!(
        name: unique_home_location_name(person.name),
        description: 'Primary home location'
      )
      LocationMembership.find_or_create_by!(location: location, person: person)
    end
  end

  def down
    # No-op: dropping memberships/locations created here could remove locations
    # that have since been used for medication inventory.
  end

  private

  def unique_home_location_name(person_name)
    base_name = "#{person_name}'s Home"
    candidate = base_name
    suffix = 2

    while Location.where('lower(name) = ?', candidate.downcase).exists?
      candidate = "#{base_name} #{suffix}"
      suffix += 1
    end

    candidate
  end
end
