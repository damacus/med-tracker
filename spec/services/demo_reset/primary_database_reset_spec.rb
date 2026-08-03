# frozen_string_literal: true

require 'rails_helper'

RSpec.describe DemoReset::PrimaryDatabaseReset do
  self.use_transactional_tests = false

  fixtures :all

  before { FixtureHouseholdSetup.apply! }

  after { restore_spec_fixtures }

  it 'atomically replaces runtime state with the committed baseline', :aggregate_failures do
    account = create_disposable_state
    metadata_counts = schema_metadata_counts

    result = described_class.new.call

    expect(result).to include(baseline: DemoBaseline::IDENTIFIER, accounts: 2, households: 1)
    expect(Account.pluck(:email)).not_to include(account.email)
    expect_disposable_state_to_be_empty
    expect(schema_metadata_counts).to eq(metadata_counts)
  end

  it 'rolls back cleanup when baseline loading fails' do
    original_emails = Account.order(:email).pluck(:email)
    reset = described_class.new(baseline_loader: -> { raise DemoBaseline::Loader::InvalidBaselineError, 'failed' })

    expect { reset.call }.to raise_error(DemoBaseline::Loader::InvalidBaselineError, 'failed')

    expect(Account.order(:email).pluck(:email)).to eq(original_emails)
  end

  it 'is idempotent and acquires the reset advisory lock' do
    connection = ActiveRecord::Base.connection
    allow(connection).to receive(:execute).and_call_original

    first_result = described_class.new(connection:).call
    second_result = described_class.new(connection:).call

    expect(second_result).to eq(first_result)
    expect(connection).to have_received(:execute).with(/pg_advisory_xact_lock/).twice
  end

  def schema_metadata_counts
    connection = ActiveRecord::Base.connection
    %w[ar_internal_metadata schema_migrations].index_with do |table|
      connection.select_value("SELECT COUNT(*) FROM #{connection.quote_table_name(table)}").to_i
    end
  end

  def create_disposable_state
    account = accounts(:admin)
    membership = account.household_memberships.first
    PushSubscription.create!(account:, endpoint: 'https://updates.push.services.mozilla.com/wpush/v2/demo',
                             p256dh: 'temporary-key', auth: 'temporary-auth')
    NativeDeviceToken.create!(account:, device_token: 'temporary-device-token', platform: 'ios')
    ApiSession.issue_for(account:, household_membership: membership)
    SecurityAuditEvent.create!(household: membership.household, actor_account: account, event_type: 'demo_mutation')
    account
  end

  def expect_disposable_state_to_be_empty
    expect(PushSubscription.count).to be_zero
    expect(NativeDeviceToken.count).to be_zero
    expect(ApiSession.count).to be_zero
    expect(SecurityAuditEvent.count).to be_zero
  end

  def restore_spec_fixtures
    connection = ActiveRecord::Base.connection
    tables = connection.tables - %w[ar_internal_metadata schema_migrations]
    quoted_tables = tables.map { |table| connection.quote_table_name(table) }.join(', ')
    connection.execute("TRUNCATE TABLE #{quoted_tables} RESTART IDENTITY CASCADE")
    fixture_names = Rails.root.glob('spec/fixtures/*.yml').map { |path| path.basename('.yml').to_s }
    SpecFixtureLoader.load(fixture_names)
    FixtureHouseholdSetup.apply!
  end
end
