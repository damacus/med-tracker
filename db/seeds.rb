# frozen_string_literal: true

# This file should ensure the existence of records required to run the application in every environment (production,
# development, test). The code here should be idempotent so that it can be executed at any point in every environment.
# The data can then be loaded with the bin/rails db:seed command (or created alongside the database with db:setup).

# Load fixtures from spec/fixtures for development/test environments
if Rails.env.local?
  Rails.logger.debug 'Loading fixtures...'

  # Load fixtures in order to respect foreign key constraints
  SpecFixtureLoader.load(
    :accounts,
    :people,
    :users,
    :medicines,
    :dosages,
    :prescriptions,
    :person_medicines,
    :medication_takes
  )

  Rails.logger.debug 'Fixtures loaded successfully!'
  Rails.logger.debug "\nYou can now login with:"
  Rails.logger.debug '  Email: damacus@example.com'
  Rails.logger.debug '  Password: password'
end
