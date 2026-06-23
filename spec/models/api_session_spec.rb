# frozen_string_literal: true

require 'rails_helper'

RSpec.describe ApiSession do
  fixtures :accounts

  describe '.issue_for' do
    let(:account) { accounts(:jane_doe) }
    let(:membership) { api_membership_for(account) }
    let(:issued_at) { Time.zone.parse('2026-04-21 10:00:00') }
    let(:expected_attributes) do
      {
        account: account,
        household_membership: membership,
        device_name: 'RSpec iPhone',
        user_agent: 'RSpec',
        last_used_at: issued_at,
        access_expires_at: issued_at + described_class::ACCESS_TOKEN_TTL,
        refresh_expires_at: issued_at + described_class::REFRESH_TOKEN_TTL
      }
    end

    it 'creates a session with digested tokens and expiry timestamps' do
      session, access_token, refresh_token = issue_session_at_issued_time

      expect(session).to have_attributes(expected_persisted_attributes(access_token, refresh_token))
      expect([access_token, refresh_token]).to all(start_with('mt_'))
    end

    it 'creates a redacted audit event without token material', :aggregate_failures do
      issued_tokens = nil

      expect do
        issued_tokens = issue_session_tokens
      end.to change { PaperTrail::Version.where(item_type: 'AuthenticationToken').count }.by(1)

      version = latest_auth_token_version
      expect_creation_audit(version)
      expect(version.object).not_to include(*issued_tokens)
    end
  end

  describe '#rotate_tokens!' do
    let(:account) { accounts(:jane_doe) }

    it 'creates a redacted rotation audit event without token material', :aggregate_failures do
      session, = described_class.issue_for(account: account, household_membership: api_membership_for(account))
      rotated_tokens = nil

      expect do
        rotated_tokens = session.rotate_tokens!
      end.to change {
        PaperTrail::Version.where(item_type: 'AuthenticationToken',
                                  event: 'auth_token/api_session/rotated').count
      }.by(1)

      version = PaperTrail::Version.where(item_type: 'AuthenticationToken').last
      expect(version.item_id).to eq(account.id)
      expect(version.object).not_to include(*rotated_tokens)
      expect(version.object).not_to include(session.reload.access_token_digest)
      expect(version.object).not_to include(session.refresh_token_digest)
    end
  end

  describe '#revoke!' do
    let(:account) { accounts(:jane_doe) }

    it 'creates a revoked audit event' do
      session, = described_class.issue_for(account: account, household_membership: api_membership_for(account))

      expect do
        session.revoke!
      end.to change {
        PaperTrail::Version.where(item_type: 'AuthenticationToken',
                                  event: 'auth_token/api_session/revoked').count
      }.by(1)

      expect(PaperTrail::Version.where(item_type: 'AuthenticationToken').last.item_id).to eq(account.id)
    end
  end

  def issue_session_tokens
    account = accounts(:jane_doe)
    described_class.issue_for(
      account: account,
      household_membership: api_membership_for(account),
      device_name: 'RSpec iPhone',
      user_agent: 'RSpec'
    ).last(2)
  end

  def issue_session_at_issued_time
    travel_to(issued_at) do
      described_class.issue_for(
        account: account,
        household_membership: membership,
        device_name: 'RSpec iPhone',
        user_agent: 'RSpec'
      )
    end
  end

  def expected_persisted_attributes(access_token, refresh_token)
    expected_attributes.merge(
      id: be_present,
      access_token_digest: described_class.digest(access_token),
      refresh_token_digest: described_class.digest(refresh_token)
    )
  end

  def api_membership_for(account)
    household = api_household
    person = api_person_for(account, household)

    household.household_memberships.create!(
      account: account,
      person: person,
      role: :owner,
      status: :active
    )
  end

  def api_household
    Household.create!(name: "API Session Spec #{SecureRandom.hex(4)}",
                      slug: "api-session-spec-#{SecureRandom.hex(4)}")
  end

  def api_person_for(account, household)
    Person.create!(
      household: household,
      account: account,
      name: 'API Session Person',
      date_of_birth: 30.years.ago.to_date,
      person_type: :adult,
      has_capacity: true
    )
  end

  def latest_auth_token_version
    PaperTrail::Version.where(item_type: 'AuthenticationToken').last
  end

  def expect_creation_audit(version)
    data = JSON.parse(version.object)
    expect(version).to have_attributes(item_id: accounts(:jane_doe).id, event: 'auth_token/api_session/created')
    expect(data).to include(
      'device_name_present' => true,
      'device_name_length' => 'RSpec iPhone'.length,
      'user_agent_hash' => Digest::SHA256.hexdigest('RSpec')
    )
  end
end
