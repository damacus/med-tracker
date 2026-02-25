# frozen_string_literal: true

# Seed default locations that should exist in all environments.
# These records are idempotent and safe to run multiple times.

Rails.logger.debug 'Seeding default locations...'

Location.find_or_create_by!(name: 'Home') do |location|
  location.description = 'Primary home location'
end

Rails.logger.debug 'Default locations seeded successfully.'
