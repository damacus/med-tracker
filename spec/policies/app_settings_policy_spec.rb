# frozen_string_literal: true

require 'rails_helper'

RSpec.describe AppSettingsPolicy, type: :policy do
  it 'denies household managers because app settings are platform-admin-only' do
    household = household_policy_member(role: :owner).fetch(:household)
    owner = household.household_memberships.owner.sole
    administrator = household_policy_member(role: :administrator, household: household)
    member = household_policy_member(role: :member, household: household)
    owner_context = AuthorizationContext.new(account: owner.account, household: household, membership: owner)

    expect(described_class.new(owner_context, :settings).show?).to be(false)
    expect(described_class.new(administrator.fetch(:context), :settings).update?).to be(false)
    expect(described_class.new(member.fetch(:context), :settings).show?).to be(false)
    expect(described_class.new(User.new, :settings).show?).to be(false)
    expect(described_class.new(nil, :settings).update?).to be(false)
  end

  it 'permits active platform admins' do
    account = Account.create!(email: 'platform-admin@example.test', status: :verified)
    PlatformAdmin.create!(account: account)
    context = AuthorizationContext.new(account: account, household: nil, membership: nil)

    expect(described_class.new(context, :settings).show?).to be(true)
    expect(described_class.new(context, :settings).update?).to be(true)
  end

  describe AppSettingsPolicy::Scope do
    it 'returns the given scope unchanged' do
      scope = User.all
      expect(described_class.new(household_policy_member(role: :owner).fetch(:context), scope).resolve).to eq(scope)
    end
  end
end
