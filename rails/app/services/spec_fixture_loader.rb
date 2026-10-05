# frozen_string_literal: true

# Utility to load RSpec fixtures directly into the database outside the test framework helpers.
class SpecFixtureLoader
  FIXTURES_PATH = Rails.root.join('spec/fixtures')
  ACCOUNT_DEPENDENT_TABLES = %w[
    account_active_session_keys
    account_identities
    account_lockouts
    account_login_change_keys
    account_login_failures
    account_otp_keys
    account_password_reset_keys
    account_recovery_codes
    account_remember_keys
    account_verification_keys
    account_webauthn_keys
    account_webauthn_user_ids
  ].freeze

  class << self
    def load(*fixture_names)
      new(fixture_names.flatten.map(&:to_s)).load
    end
  end

  def initialize(fixture_names)
    @fixture_names = fixture_names
  end

  def load
    ActiveRecord::FixtureSet.reset_cache
    clear_account_dependent_tables
    # Load fixtures - foreign keys are deferrable so order doesn't matter within a transaction
    ActiveRecord::FixtureSet.create_fixtures(FIXTURES_PATH, fixture_names)
  end

  private

  attr_reader :fixture_names

  def clear_account_dependent_tables
    return unless fixture_names.include?('accounts')

    connection = ActiveRecord::Base.connection
    ACCOUNT_DEPENDENT_TABLES.each do |table_name|
      next unless connection.data_source_exists?(table_name)

      connection.delete("DELETE FROM #{connection.quote_table_name(table_name)}")
    end
  end
end
