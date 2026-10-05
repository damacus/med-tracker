# frozen_string_literal: true

module DemoBaseline
  IDENTIFIER = 'weekly-demo-v1'
  HOUSEHOLD_SLUG = 'demo-household'
  OWNER_EMAIL = 'demo.owner@example.com'
  FIXTURES_PATH = Rails.root.join('db/demo')
  FIXTURE_NAMES = %w[
    accounts
    households
    people
    users
    household_memberships
    carer_relationships
    person_access_grants
    locations
    location_memberships
    medications
    schedules
    person_medications
    medication_takes
  ].freeze
end
