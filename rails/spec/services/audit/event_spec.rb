# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Audit::Event do
  fixtures :accounts, :people, :users

  let(:account) { accounts(:admin) }
  let(:user) { users(:admin) }
  let(:household) { user.person.household || create(:household) }
  let(:membership) do
    household.household_memberships.find_or_create_by!(account: account) do |record|
      record.person = user.person
      record.role = :owner
      record.status = :active
    end
  end

  before do
    Current.account = account
    Current.household = household
    Current.membership = membership
    Current.audit_context = {
      actor_account_id: account.id,
      actor_user_id: user.id,
      actor_membership_id: membership.id,
      active_role: membership.role,
      policy_class: 'MedicationPolicy',
      policy_query: 'update?'
    }
  end

  after { Current.reset }

  it 'writes normalized context and recursively redacts secret metadata', :aggregate_failures do
    event = record_event

    expect_event_identity(event)
    expect_event_context(event)
    expect_redacted_metadata(event)
  end

  it 'does not swallow persistence failures' do
    allow(SecurityAuditEvent).to receive(:create!).and_raise(ActiveRecord::StatementInvalid, 'audit unavailable')

    expect do
      described_class.record!(household: household, event_type: 'security.test')
    end.to raise_error(ActiveRecord::StatementInvalid, 'audit unavailable')
  end

  def record_event
    described_class.record!(
      household: household,
      event_type: 'security.test',
      metadata: {
        outcome: 'success',
        access_token: 'raw-access-token',
        nested: { password: 'raw-password', safe_value: 'kept' }
      }
    )
  end

  def expect_event_identity(event)
    expect(event).to have_attributes(
      actor_account: account,
      actor_membership: membership,
      event_type: 'security.test'
    )
  end

  def expect_event_context(event)
    expect(event.audit_context).to include(
      'active_role' => 'owner',
      'policy_class' => 'MedicationPolicy',
      'policy_query' => 'update?'
    )
  end

  def expect_redacted_metadata(event)
    expect(event.metadata).to include('outcome' => 'success', 'nested' => include('safe_value' => 'kept'))
    expect(event.metadata.to_json).not_to include('raw-access-token', 'raw-password')
  end
end
