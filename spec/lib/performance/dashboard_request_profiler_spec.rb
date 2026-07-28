# frozen_string_literal: true

require 'rails_helper'
require 'performance/dashboard_request_profiler'

RSpec.describe Performance::DashboardRequestProfiler do
  let(:account) do
    Account.create!(
      email: "dashboard-profile-#{SecureRandom.hex(4)}@example.com",
      password_hash: RodauthApp.rodauth.allocate.password_hash('password'),
      status: :verified
    )
  end
  let!(:household) do
    Household.create_with_owner!(
      name: 'Profile Household',
      owner_account: account,
      owner_person_attributes: {
        name: 'Profile Owner',
        date_of_birth: 30.years.ago.to_date,
        person_type: :adult,
        has_capacity: true
      }
    )
  end
  let(:profile_user) do
    User.create!(
      person: household.people.find_by!(account:),
      email_address: account.email,
      password: 'password',
      active: true
    )
  end

  it 'profiles complete authenticated dashboard requests', :aggregate_failures do
    profile_user
    result = described_class.new(configuration(warmup_iterations: 1, measured_iterations: 2)).run

    expect(result.request_path).to eq("/households/#{household.slug}/dashboard?dashboard_person_id=all")
    expect(result.response_status).to eq(200)
    expect(result.measurements.size).to eq(2)
    expect(result.measurements.map(&:elapsed_ms)).to all(be_positive)
    expect(result.measurements.map(&:sql_count)).to all(be_positive)
    expect(result.measurements.map(&:allocations)).to all(be_positive)
  end

  it 'renders a stable percentile summary' do
    expect(deterministic_result.to_markdown).to eq(expected_summary)
  end

  it 'rejects invalid iteration counts' do
    expect do
      described_class.new(configuration(warmup_iterations: 0, measured_iterations: 0))
    end.to raise_error(ArgumentError, 'measured_iterations must be at least 1')
  end

  def configuration(overrides = {})
    attributes = {
      application: Rails.application,
      profile_email: account.email,
      password: 'password',
      selected_person_id: DashboardPresenter::ALL_FAMILY_PERSON_ID,
      artifact_path: 'tmp/dashboard-profile.vernier.json.gz',
      warmup_iterations: 1,
      measured_iterations: 2,
      host: 'localhost'
    }
    attributes.merge!(overrides)
    described_class::Configuration.new(**attributes)
  end

  def deterministic_result
    described_class::Result.new(
      captured_at: Time.utc(2026, 7, 28, 12, 30),
      profile_email: 'admin@example.com',
      household_slug: 'profile-household',
      selected_person_id: 'all',
      artifact_path: 'tmp/dashboard-profile.vernier.json.gz',
      request_path: '/households/profile-household/dashboard?dashboard_person_id=all',
      warmup_iterations: 2,
      measured_iterations: 3,
      response_status: 200,
      measurements: deterministic_measurements
    )
  end

  def deterministic_measurements
    [
      described_class::Measurement.new(elapsed_ms: 10.126, sql_count: 8, allocations: 100),
      described_class::Measurement.new(elapsed_ms: 15.555, sql_count: 10, allocations: 150),
      described_class::Measurement.new(elapsed_ms: 20.994, sql_count: 12, allocations: 200)
    ]
  end

  def expected_summary
    <<~MARKDOWN
      # Dashboard Request Profile

      - Captured at: 2026-07-28T12:30:00Z
      - Account: admin@example.com
      - Household: profile-household
      - Selected person: all
      - Request path: /households/profile-household/dashboard?dashboard_person_id=all
      - Vernier artifact: tmp/dashboard-profile.vernier.json.gz
      - Warmup iterations: 2
      - Measured iterations: 3
      - HTTP status: 200
      - Request latency p50: 15.56ms
      - Request latency p95: 20.99ms
      - SQL queries p50: 10
      - SQL queries p95: 12
      - Allocations p50: 150
      - Allocations p95: 200
    MARKDOWN
  end
end
