# frozen_string_literal: true

medications_file = Rails.root.join('db/seeds/medications.yml')
medications_data = YAML.load_file(medications_file)

default_location = Location.find_or_create_by!(name: 'Home') do |loc|
  loc.description = 'Primary home location'
end

medications_data.each do |attrs|
  medication = Medication.find_or_initialize_by(name: attrs['name'])
  medication.location ||= default_location
  medication.update!(
    dosage_amount: attrs['dosage_amount'],
    dosage_unit: attrs['dosage_unit'],
    current_supply: attrs['current_supply'],
    supply_at_last_restock: attrs['current_supply'],
    reorder_threshold: attrs['reorder_threshold'],
    description: attrs['description'],
    warnings: attrs['warnings']
  )
end

Rails.logger.debug { "Seeded #{medications_data.size} medications." }
