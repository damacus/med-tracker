# frozen_string_literal: true

require 'rails_helper'

load Rails.root.join('db/migrate/20260911121000_bound_api_app_token_lifetimes.rb') unless
  defined?(BoundApiAppTokenLifetimes)

RSpec.describe BoundApiAppTokenLifetimes do
  fixtures :all

  it 'backfills existing tokens from issuance, including already expired leap-day tokens' do
    account = users(:admin).person.account
    household = Household.create!(name: 'Token migration', slug: 'token-migration')
    membership = household.household_memberships.create!(account: account, role: :owner, status: :active)
    token = nil
    travel_to(Time.zone.parse('2024-02-29 10:00:00')) do
      token, = ApiAppToken.issue_for(account: account, household_membership: membership,
                                     name: 'Before expiry migration')
    end
    allow(ENV).to receive(:fetch).and_call_original
    allow(ENV).to receive(:fetch).with('API_APP_TOKEN_MAX_AGE_MONTHS', '12').and_return('12')

    described_class.new.down
    described_class.new.up

    expect(token.reload.expires_at).to eq(Time.zone.parse('2025-02-28 10:00:00'))
    expect(token).not_to be_unexpired
  end
end
