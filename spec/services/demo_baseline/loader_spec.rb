# frozen_string_literal: true

require 'rails_helper'

RSpec.describe DemoBaseline::Loader do
  self.use_transactional_tests = false

  around do |example|
    ActiveRecord::Base.connection.transaction(requires_new: true) do
      truncate_runtime_tables
      example.run
      raise ActiveRecord::Rollback
    end
  end

  it 'loads the documented synthetic household into an empty migrated database', :aggregate_failures do
    result = described_class.load!

    expect_baseline_summary(result)
    expect_demo_records
  end

  it 'contains no delivery registrations, API credentials, or uploads', :aggregate_failures do
    described_class.load!

    expect(PushSubscription.count).to be_zero
    expect(NativeDeviceToken.count).to be_zero
    expect(ApiAppToken.count).to be_zero
    expect(ApiSession.count).to be_zero
    expect(ActiveStorage::Attachment.count).to be_zero
    expect(ActiveStorage::Blob.count).to be_zero
  end

  def expect_baseline_summary(result)
    expect(result).to eq(
      baseline: DemoBaseline::IDENTIFIER,
      accounts: 2,
      households: 1,
      people: 3,
      medications: 2,
      schedules: 1,
      as_needed_medications: 1,
      medication_takes: 2
    )
  end

  def expect_demo_records
    expect_demo_accounts
    expect_demo_household_access
    expect_demo_medication_scenarios
  end

  def expect_demo_accounts
    expect(Account.order(:email).pluck(:email)).to eq(%w[demo.carer@example.com demo.owner@example.com])
    owner = Account.find_by!(email: DemoBaseline::OWNER_EMAIL)
    expect(ApiAuthState.password_authenticated?(owner, 'password')).to be(true)
  end

  def expect_demo_household_access
    expect(Household.find_by!(slug: DemoBaseline::HOUSEHOLD_SLUG).household_memberships.pluck(:role)).to match_array(
      %w[owner member]
    )
    expect(Person.all.map(&:person_type)).to include('adult', 'minor')
    expect(PersonAccessGrant.active.count).to eq(5)
  end

  def expect_demo_medication_scenarios
    expect(Schedule.first).to be_schedule_type_daily
    expect(PersonMedication.first).to be_as_needed
    expect(MedicationTake.pluck(:taken_at)).to all(satisfy { |taken_at| taken_at.to_date == Time.zone.today })
  end

  def truncate_runtime_tables
    connection = ActiveRecord::Base.connection
    excluded_tables = %w[ar_internal_metadata schema_migrations]
    tables = connection.tables - excluded_tables
    quoted_tables = tables.map { |table| connection.quote_table_name(table) }.join(', ')
    connection.execute("TRUNCATE TABLE #{quoted_tables} RESTART IDENTITY CASCADE")
  end
end
