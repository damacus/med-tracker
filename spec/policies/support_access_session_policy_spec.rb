# frozen_string_literal: true

require 'rails_helper'

RSpec.describe SupportAccessSessionPolicy, type: :policy do
  it 'permits active platform admins to create and destroy support sessions' do
    account = Account.create!(email: 'active-platform-admin@example.test', status: :verified)
    platform_admin = PlatformAdmin.create!(account: account)
    support_session = build_support_session(platform_admin, 'active-support-policy-household')
    policy = described_class.new(platform_context(account), support_session)

    expect(policy.create?).to be(true)
    expect(policy.destroy?).to be(true)
  end

  it 'denies support-session creation for non-operational households' do
    account = Account.create!(email: 'unavailable-support-policy-admin@example.test', status: :verified)
    platform_admin = PlatformAdmin.create!(account: account)

    %i[held offboarded purging purged].each do |lifecycle_state|
      support_session = build_support_session(
        platform_admin,
        "#{lifecycle_state}-support-policy-household",
        lifecycle_state: lifecycle_state
      )

      expect(described_class.new(platform_context(account), support_session).create?).to be(false)
    end
  end

  it 'denies disabled, missing, and nil platform admin contexts' do
    disabled_account = Account.create!(email: 'disabled-platform-admin@example.test', status: :verified)
    regular_account = Account.create!(email: 'regular-platform-user@example.test', status: :verified)
    PlatformAdmin.create!(account: disabled_account, status: :disabled)

    expect(described_class.new(platform_context(disabled_account), SupportAccessSession).create?).to be(false)
    expect(described_class.new(platform_context(regular_account), SupportAccessSession).destroy?).to be(false)
    expect(described_class.new(nil, SupportAccessSession).create?).to be(false)
  end

  describe SupportAccessSessionPolicy::Scope do
    it 'returns only the active platform admin support sessions' do
      active_account = Account.create!(email: 'scope-platform-admin@example.test', status: :verified)
      other_account = Account.create!(email: 'other-platform-admin@example.test', status: :verified)
      active_admin = PlatformAdmin.create!(account: active_account)
      other_admin = PlatformAdmin.create!(account: other_account)
      active_session = create_support_session(active_admin, 'scope-household-a')
      create_support_session(other_admin, 'scope-household-b')

      resolved = described_class.new(platform_context(active_account), SupportAccessSession.all).resolve

      expect(resolved).to contain_exactly(active_session)
    end

    it 'returns no sessions for disabled, missing, and nil platform admin contexts' do
      disabled_account = Account.create!(email: 'disabled-scope-platform-admin@example.test', status: :verified)
      regular_account = Account.create!(email: 'regular-scope-platform-user@example.test', status: :verified)
      PlatformAdmin.create!(account: disabled_account, status: :disabled)
      create_support_session(PlatformAdmin.create!(account: Account.create!(email: 'visible-admin@example.test',
                                                                            status: :verified)),
                             'scope-household-c')

      expect(described_class.new(platform_context(disabled_account), SupportAccessSession.all).resolve).to be_empty
      expect(described_class.new(platform_context(regular_account), SupportAccessSession.all).resolve).to be_empty
      expect(described_class.new(nil, SupportAccessSession.all).resolve).to be_empty
    end
  end

  def platform_context(account)
    AuthorizationContext.new(account: account, household: nil, membership: nil)
  end

  def create_support_session(platform_admin, slug)
    build_support_session(platform_admin, slug).tap(&:save!)
  end

  def build_support_session(platform_admin, slug, lifecycle_state: :active)
    SupportAccessSession.new(
      platform_admin: platform_admin,
      household: Household.create!(name: slug.titleize, slug: slug, lifecycle_state: lifecycle_state),
      reason: 'Investigate support access policy',
      mfa_verified_at: Time.current,
      starts_at: 1.minute.ago,
      expires_at: 10.minutes.from_now
    )
  end
end
