# frozen_string_literal: true

require 'rails_helper'

RSpec.describe ApiHouseholdSelectionGrant do
  let(:account) { create_account_with_household('selection-owner') }
  let(:membership) { account.household_memberships.active.sole }

  it 'issues a five-minute grant while storing only the token digest' do
    issued_at = Time.zone.parse('2026-07-17 10:00:00')

    travel_to(issued_at) do
      grant, token = issue_mfa_grant

      expect_issued_grant(grant, token, issued_at)
    end
  end

  it 'rejects expired and already-used grants' do
    expired_grant, expired_token = described_class.issue_for(account: account)
    expired_grant.update!(expires_at: 1.minute.ago)

    expect do
      described_class.select_household(token: expired_token, household_id: membership.household_id)
    end.to raise_error(described_class::InvalidGrant)

    used_grant, used_token = described_class.issue_for(account: account)
    used_grant.update!(used_at: Time.current)

    expect do
      described_class.select_household(token: used_token, household_id: membership.household_id)
    end.to raise_error(described_class::InvalidGrant)
  end

  it 'rejects a household membership owned by another account' do
    other_account = create_account_with_household('selection-other')
    _, token = described_class.issue_for(account: account)

    expect do
      described_class.select_household(
        token: token,
        household_id: other_account.household_memberships.active.sole.household_id
      )
    end.to raise_error(described_class::InvalidGrant)
  end

  it 'consumes a valid grant exactly once and preserves its MFA and device state' do
    grant, token = issue_mfa_grant

    result = described_class.select_household(token: token, household_id: membership.household_id)

    expect_preserved_session(result)
    expect(grant.reload.used_at).to be_present
    expect do
      described_class.select_household(token: token, household_id: membership.household_id)
    end.to raise_error(described_class::InvalidGrant)
  end

  def issue_mfa_grant
    described_class.issue_for(
      account: account,
      oidc_mfa_verified: true,
      mfa_verified_at: Time.current,
      device_name: 'Android',
      user_agent: 'RSpec'
    )
  end

  def expect_issued_grant(grant, token, issued_at)
    expect(grant).to have_attributes(
      token_digest: described_class.digest(token),
      expires_at: issued_at + described_class::TOKEN_TTL,
      oidc_mfa_verified: true,
      device_name: 'Android',
      user_agent: 'RSpec'
    )
    expect(grant.token_digest).not_to eq(token)
  end

  def expect_preserved_session(result)
    expect(result.api_session).to have_attributes(
      account: account,
      household_membership: membership,
      oidc_mfa_verified: true,
      device_name: 'Android',
      user_agent: 'RSpec'
    )
  end

  def create_account_with_household(prefix)
    account = Account.create!(email: "#{prefix}-#{SecureRandom.hex(4)}@example.test", status: :verified)
    household = create_household(account, prefix)
    create_user(account, household)
    account
  end

  def create_household(account, prefix)
    Household.create_with_owner!(
      name: "#{prefix} #{SecureRandom.hex(4)}",
      owner_account: account,
      owner_person_attributes: {
        name: 'Selection Owner',
        date_of_birth: 30.years.ago.to_date,
        person_type: :adult,
        has_capacity: true
      }
    )
  end

  def create_user(account, household)
    User.create!(
      person: household.people.find_by!(account: account),
      email_address: account.email,
      password: 'password',
      active: true
    )
  end
end
