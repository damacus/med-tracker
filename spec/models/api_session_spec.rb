# frozen_string_literal: true

require 'rails_helper'

RSpec.describe ApiSession do
  fixtures :accounts

  describe '.issue_for' do
    let(:account) { accounts(:jane_doe) }
    let(:issued_at) { Time.zone.parse('2026-04-21 10:00:00') }
    let(:expected_attributes) do
      {
        account: account,
        device_name: 'RSpec iPhone',
        user_agent: 'RSpec',
        last_used_at: issued_at,
        access_expires_at: issued_at + described_class::ACCESS_TOKEN_TTL,
        refresh_expires_at: issued_at + described_class::REFRESH_TOKEN_TTL
      }
    end

    it 'creates a session with digested tokens and expiry timestamps' do
      travel_to(issued_at) do
        session, access_token, refresh_token = described_class.issue_for(
          account: account,
          device_name: 'RSpec iPhone',
          user_agent: 'RSpec'
        )

        expect(session).to have_attributes(
          expected_attributes.merge(
            id: be_present,
            access_token_digest: described_class.digest(access_token),
            refresh_token_digest: described_class.digest(refresh_token)
          )
        )
        expect([access_token, refresh_token]).to all(start_with('mt_'))
      end
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
      session, = described_class.issue_for(account: account)
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
      session, = described_class.issue_for(account: account)

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
    described_class.issue_for(
      account: accounts(:jane_doe),
      device_name: 'RSpec iPhone',
      user_agent: 'RSpec'
    ).last(2)
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
