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
  end
end
