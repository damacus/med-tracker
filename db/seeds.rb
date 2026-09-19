# frozen_string_literal: true

# This file should ensure the existence of records required to run the application in every environment (production,
# development, test). The code here should be idempotent so that it can be executed at any point in every environment.
# The data can then be loaded with the bin/rails db:seed command (or created alongside the database with db:setup).

seeded_at = Time.current
android_client_ids = %w[io.damacus.medtracker io.damacus.medtracker.debug io.damacus.medtracker.staging]
OauthApplication.insert_all(
  android_client_ids.map do |client_id|
    {
      name: 'MedTracker Android',
      client_id:,
      redirect_uri: "#{client_id}:/oauth2redirect",
      scopes: 'medtracker offline_access',
      client_kind: 'mobile',
      token_endpoint_auth_method: 'none',
      created_at: seeded_at,
      updated_at: seeded_at
    }
  end,
  unique_by: :index_oauth_applications_on_client_id
)

# Seed reference medicine data in all environments
if Rails.env.local?
  PaperTrail.request(enabled: false) do
    Rails.logger.debug 'Seeding default locations...'
    load Rails.root.join('db/seeds/seed_locations.rb')

    Rails.logger.debug 'Seeding medicines...'
    load Rails.root.join('db/seeds/seed_medications.rb')

    Rails.logger.debug 'Loading fixtures...'

    # Load fixtures in order to respect foreign key constraints
    SpecFixtureLoader.load(
      :accounts,
      :account_otp_keys,
      :people,
      :users,
      :locations,
      :location_memberships,
      :medications,
      :dosages,
      :schedules,
      :person_medications,
      :carer_relationships,
      :medication_takes
    )
    FixtureHouseholdSetup.apply!

    Rails.logger.debug 'Fixtures loaded successfully!'
    Rails.logger.debug "\nYou can now login with:"
    Rails.logger.debug '  Email: jane.doe@example.com (no 2FA)'
    Rails.logger.debug '  Password: password'
    Rails.logger.debug '  Note: damacus@example.com has TOTP enabled'
  end
else
  Rails.logger.debug 'Seeding default locations...'
  load Rails.root.join('db/seeds/seed_locations.rb')

  Rails.logger.debug 'Seeding medicines...'
  load Rails.root.join('db/seeds/seed_medications.rb')
end

if Rails.env.production?
  # In production, invite initial users from db/seeds/users.yml.
  # Operators should replace placeholder emails before first deploy, or mount a
  # custom users.yml via a k8s ConfigMap volume at /app/db/seeds/users.yml.
  Rails.logger.debug 'Seeding initial users via invitations...'
  load Rails.root.join('db/seeds/seed_users.rb')
end
