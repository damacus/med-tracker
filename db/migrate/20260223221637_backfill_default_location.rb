# frozen_string_literal: true

class BackfillDefaultLocation < ActiveRecord::Migration[8.1]
  def up
    location = Location.find_or_create_by!(name: 'Home') do |loc|
      loc.description = 'Default home location'
    end

    Medicine.where(location_id: nil).update_all(location_id: location.id) # rubocop:disable Rails/SkipsModelValidations

    person_ids = Prescription.distinct.pluck(:person_id) |
                 PersonMedicine.distinct.pluck(:person_id)

    person_ids.each do |person_id|
      LocationMembership.find_or_create_by!(location: location, person_id: person_id)
    end
  end

  def down
    location = Location.find_by(name: 'Home')
    return unless location

    LocationMembership.where(location: location).delete_all
    Medicine.where(location: location).update_all(location_id: nil) # rubocop:disable Rails/SkipsModelValidations
    location.destroy
  end
end
