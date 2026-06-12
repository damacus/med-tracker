# frozen_string_literal: true

class BackfillDefaultLocation < ActiveRecord::Migration[8.1]
  def up
    default_location = Location.create!(
      name: unique_home_location_name('Medication'),
      description: 'Default medication location'
    )

    Medicine.where(location_id: nil).update_all(location_id: default_location.id) # rubocop:disable Rails/SkipsModelValidations

    person_ids = Prescription.distinct.pluck(:person_id) |
                 PersonMedicine.distinct.pluck(:person_id)

    person_ids.each do |person_id|
      person = Person.find(person_id)
      location = Location.create!(
        name: unique_home_location_name(person.name),
        description: 'Primary home location'
      )
      LocationMembership.find_or_create_by!(location: location, person_id: person_id)
    end
  end

  def down
    # No-op: these isolated default locations may have been selected for later
    # medication inventory, so do not delete them during rollback.
  end

  private

  def unique_home_location_name(owner_name)
    base_name = "#{owner_name}'s Home"
    candidate = base_name
    suffix = 2

    while Location.where('lower(name) = ?', candidate.downcase).exists?
      candidate = "#{base_name} #{suffix}"
      suffix += 1
    end

    candidate
  end
end
