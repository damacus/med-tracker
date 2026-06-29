# frozen_string_literal: true

require 'rails_helper'

RSpec.describe SupportAccessSession do
  it 'requires a platform admin, target household, reason, and MFA timestamp' do
    session = described_class.new

    expect(session).not_to be_valid
    expect(session.errors[:platform_admin]).to include('must exist')
    expect(session.errors[:household]).to include('must exist')
    expect(session.errors[:reason]).to include("can't be blank")
    expect(session.errors[:mfa_verified_at]).to include("can't be blank")
  end

  it 'recognizes active support sessions within their time window' do
    platform_admin = described_class_platform_admin
    household = Household.create!(name: 'Support Household', slug: 'support-household')
    session = described_class.create!(
      platform_admin: platform_admin,
      household: household,
      reason: 'Investigate invitation delivery failure',
      mfa_verified_at: Time.current,
      starts_at: 1.minute.ago,
      expires_at: 10.minutes.from_now
    )

    expect(session).to be_active
  end

  it 'does not treat ended support sessions as active' do
    session = build_support_access_session(ended_at: Time.current)

    expect(session).not_to be_active
  end

  it 'does not treat future support sessions as active' do
    session = build_support_access_session(starts_at: 5.minutes.from_now, expires_at: 30.minutes.from_now)

    expect(session).not_to be_active
  end

  it 'does not treat expired support sessions as active' do
    session = build_support_access_session(starts_at: 30.minutes.ago, expires_at: 1.minute.ago)

    expect(session).not_to be_active
  end

  it 'defaults the support access time window before validation' do
    session = described_class.new(
      platform_admin: described_class_platform_admin,
      household: Household.create!(name: 'Default Window Household', slug: 'default-window-household'),
      reason: 'Check support session defaults',
      mfa_verified_at: Time.current
    )

    expect(session).to be_valid
    expect(session.starts_at).to be_present
    expect(session.expires_at).to be_within(1.second).of(session.starts_at + 30.minutes)
  end

  it 'requires expiry to be after the start time' do
    timestamp = Time.current
    session = build_support_access_session(starts_at: timestamp, expires_at: timestamp)

    expect(session).not_to be_valid
    expect(session.errors[:expires_at]).to include('must be after starts at')
  end

  def described_class_platform_admin
    account = Account.create!(email: 'support-admin@example.test', status: :verified)
    PlatformAdmin.create!(account: account)
  end

  def build_support_access_session(attributes = {})
    platform_admin = described_class_platform_admin
    household_suffix = SecureRandom.hex(4)
    household = Household.create!(
      name: "Support Household #{household_suffix}",
      slug: "support-household-#{household_suffix}"
    )

    described_class.new({
      platform_admin: platform_admin,
      household: household,
      reason: 'Investigate invitation delivery failure',
      mfa_verified_at: Time.current,
      starts_at: 1.minute.ago,
      expires_at: 10.minutes.from_now
    }.merge(attributes))
  end
end
