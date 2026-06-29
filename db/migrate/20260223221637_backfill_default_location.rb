# frozen_string_literal: true

class BackfillDefaultLocation < ActiveRecord::Migration[8.1]
  class MigrationLocation < ActiveRecord::Base
    self.table_name = 'locations'
  end

  class MigrationLocationMembership < ActiveRecord::Base
    self.table_name = 'location_memberships'
  end

  class MigrationMedicine < ActiveRecord::Base
    self.table_name = 'medicines'
  end

  class MigrationPrescription < ActiveRecord::Base
    self.table_name = 'prescriptions'
  end

  class MigrationPersonMedicine < ActiveRecord::Base
    self.table_name = 'person_medicines'
  end

  def up
    location = MigrationLocation.find_or_create_by!(name: 'Home') do |loc|
      loc.description = 'Default home location'
    end

    MigrationMedicine.where(location_id: nil).update_all(location_id: location.id) # rubocop:disable Rails/SkipsModelValidations

    person_ids = MigrationPrescription.distinct.pluck(:person_id) |
                 MigrationPersonMedicine.distinct.pluck(:person_id)

    person_ids.each do |person_id|
      MigrationLocationMembership.find_or_create_by!(location_id: location.id, person_id: person_id)
    end
  end

  def down
    location = MigrationLocation.find_by(name: 'Home')
    return unless location

    MigrationLocationMembership.where(location_id: location.id).delete_all
    MigrationMedicine.where(location_id: location.id).update_all(location_id: nil) # rubocop:disable Rails/SkipsModelValidations
    location.destroy
  end
end
