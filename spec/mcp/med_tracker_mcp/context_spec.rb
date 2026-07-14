# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedTrackerMcp::Context do
  fixtures :accounts, :people, :users, :households

  describe '.resolve!' do
    it 'returns a token-scoped server context for a valid app token' do
      context = described_class.resolve!(request_for(raw_token))

      expect(context.to_h).to include(
        account: account,
        account_id: account.id,
        household: household,
        household_id: household.id,
        membership: membership,
        membership_id: membership.id,
        request_id: 'mcp-request-123',
        remote_ip: '203.0.113.10'
      )
      expect(context.to_h.fetch(:api_credential)).to eq(app_token)
      expect(context.to_h.values).not_to include(raw_token)
    end

    it 'binds tenant state only while yielding through the context' do
      context = described_class.resolve!(request_for(raw_token))
      Current.reset

      current_state = nil
      context.with_current do
        current_state = [Current.account, Current.household, Current.membership, Current.request_id]
      end

      expect(current_state).to eq([account, household, membership, 'mcp-request-123'])
      expect([Current.account, Current.household, Current.membership, Current.request_id]).to all(be_nil)
    end

    it 'rejects requests without bearer tokens' do
      expect_authentication_failure(request_for(nil))
    end

    it 'rejects unknown bearer tokens' do
      expect_authentication_failure(request_for('not-a-real-token'))
    end

    it 'rejects revoked app tokens' do
      app_token.revoke!(audit_context: { request_id: 'revoke-request' })

      expect_authentication_failure(request_for(raw_token))
    end

    it 'rejects locked accounts' do
      AccountLockout.create!(
        account: account,
        key: 'mcp-lockout',
        deadline: 1.hour.from_now,
        created_at: Time.current,
        updated_at: Time.current
      )

      expect_authentication_failure(request_for(raw_token))
    end

    it 'rejects inactive memberships' do
      raw_token
      membership.update!(status: :suspended)

      expect_authentication_failure(request_for(raw_token))
    end

    it 'rejects tokens with stale membership permissions' do
      raw_token
      membership.update!(permissions_version: membership.permissions_version + 1)

      expect_authentication_failure(request_for(raw_token))
    end

    it 'rejects tokens for every non-operational household lifecycle state' do
      raw_token

      %i[held offboarded purging purged].each do |lifecycle_state|
        household.update_columns(lifecycle_state: lifecycle_state)

        expect_authentication_failure(request_for(raw_token))
      end
    end
  end

  def request_for(token)
    Struct.new(:headers, :request_id, :remote_ip, keyword_init: true).new(
      headers: token ? { 'Authorization' => "Bearer #{token}" } : {},
      request_id: 'mcp-request-123',
      remote_ip: '203.0.113.10'
    )
  end

  def expect_authentication_failure(request)
    expect do
      described_class.resolve!(request)
    end.to raise_error(described_class::AuthenticationError) { |error|
      expect(error).to have_attributes(
        status: :unauthorized,
        code: 'unauthorized',
        message: 'Authentication required'
      )
    }
  end

  def account
    accounts(:jane_doe)
  end

  def household
    households(:fixture_household)
  end

  def person
    people(:jane)
  end

  def membership
    @membership ||= household.household_memberships.find_or_create_by!(account: account) do |household_membership|
      household_membership.person = person
      household_membership.role = :member
      household_membership.status = :active
    end
  end

  def app_token
    token_pair.first
  end

  def raw_token
    token_pair.last
  end

  def token_pair
    @token_pair ||= ApiAppToken.issue_for(
      account: account,
      household_membership: membership,
      name: 'RSpec MCP token',
      audit_context: { request_id: 'issue-request' }
    )
  end
end
