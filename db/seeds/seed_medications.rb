# frozen_string_literal: true

medications_file = Rails.root.join('db/seeds/medications.yml')
medications_data = YAML.load_file(medications_file)

household_slug = Rails.env.local? ? 'fixture-household' : 'seed-household'
household = Household.find_or_initialize_by(slug: household_slug)
household.id ||= ActiveRecord::FixtureSet.identify(:fixture_household) if Rails.env.local?
household.name ||= Rails.env.local? ? 'Fixture Household' : 'Seed Household'
household.status ||= 'active'
household.timezone ||= Time.zone.name
household.subscription_plan ||= 'free'
household.save!

default_location = household.locations.find_or_create_by!(name: 'Home') do |loc|
  loc.description = 'Primary home location'
end

medications_data.each do |attrs|
  medication = household.medications.find_or_initialize_by(name: attrs['name'])
  medication.location ||= default_location
  medication.update!(
    category: attrs['category'],
    dosage_amount: attrs['dosage_amount'],
    dosage_unit: attrs['dosage_unit'],
    default_schedule_type: attrs.fetch('default_schedule_type', medication.default_schedule_type),
    current_supply: attrs['current_supply'],
    supply_at_last_restock: attrs['current_supply'],
    reorder_threshold: attrs['reorder_threshold'],
    description: attrs['description'],
    warnings: attrs['warnings']
  )
end

Rails.logger.debug { "Seeded #{medications_data.size} medications." }
