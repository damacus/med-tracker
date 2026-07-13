# frozen_string_literal: true

require 'rails_helper'

RSpec.describe HouseholdRetentionHoldPolicy, type: :policy do
  it 'permits only active platform administrators to place and release holds' do
    active_account = Account.create!(email: 'hold-policy-admin@example.test', status: :verified)
    disabled_account = Account.create!(email: 'hold-policy-disabled@example.test', status: :verified)
    regular_account = Account.create!(email: 'hold-policy-regular@example.test', status: :verified)
    PlatformAdmin.create!(account: active_account)
    PlatformAdmin.create!(account: disabled_account, status: :disabled)

    active_policy = described_class.new(platform_context(active_account), HouseholdRetentionHold)
    disabled_policy = described_class.new(platform_context(disabled_account), HouseholdRetentionHold)
    regular_policy = described_class.new(platform_context(regular_account), HouseholdRetentionHold)

    expect(active_policy.create?).to be(true)
    expect(active_policy.update?).to be(true)
    expect(disabled_policy.create?).to be(false)
    expect(regular_policy.update?).to be(false)
  end

  def platform_context(account)
    AuthorizationContext.new(account: account, household: nil, membership: nil)
  end
end
