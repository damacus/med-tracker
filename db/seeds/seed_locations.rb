# frozen_string_literal: true

# Seed default locations that should exist in all environments.
# These records are idempotent and safe to run multiple times.

Rails.logger.debug 'Seeding default locations...'

household_slug = Rails.env.local? ? 'fixture-household' : 'seed-household'
household = Household.find_or_initialize_by(slug: household_slug)
household.id ||= ActiveRecord::FixtureSet.identify(:fixture_household) if Rails.env.local?
household.name ||= Rails.env.local? ? 'Fixture Household' : 'Seed Household'
household.status ||= 'active'
household.timezone ||= Time.zone.name
household.subscription_plan ||= 'free'
household.save!

household.locations.find_or_create_by!(name: 'Home') do |location|
  location.description = 'Primary home location'
end

Rails.logger.debug 'Default locations seeded successfully.'
