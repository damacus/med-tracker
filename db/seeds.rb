# frozen_string_literal: true

# This file should ensure the existence of records required to run the application in every environment (production,
# development, test). The code here should be idempotent so that it can be executed at any point in every environment.
# The data can then be loaded with the bin/rails db:seed command (or created alongside the database with db:setup).

# Seed reference medicine data in all environments
Rails.logger.debug 'Seeding medicines...'
load Rails.root.join('db/seeds/seed_medicines.rb')

if Rails.env.production?
  # In production, invite initial users from db/seeds/users.yml.
  # Operators should replace placeholder emails before first deploy, or mount a
  # custom users.yml via a k8s ConfigMap volume at /app/db/seeds/users.yml.
  Rails.logger.debug 'Seeding initial users via invitations...'
  load Rails.root.join('db/seeds/seed_users.rb')
end

# Load fixtures from spec/fixtures for development/test environments
if Rails.env.local?
  Rails.logger.debug 'Loading fixtures...'

  # Load fixtures in order to respect foreign key constraints
  SpecFixtureLoader.load(
    :accounts,
    :account_otp_keys,
    :people,
    :users,
    :locations,
    :location_memberships,
    :medicines,
    :dosages,
    :prescriptions,
    :person_medicines,
    :carer_relationships,
    :medication_takes
  )

  Rails.logger.debug 'Fixtures loaded successfully!'
  Rails.logger.debug "\nYou can now login with:"
  Rails.logger.debug '  Email: jane.doe@example.com (no 2FA)'
  Rails.logger.debug '  Password: password'
  Rails.logger.debug '  Note: damacus@example.com has TOTP enabled'
end
