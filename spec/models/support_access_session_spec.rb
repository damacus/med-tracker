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

  def described_class_platform_admin
    account = Account.create!(email: 'support-admin@example.test', status: :verified)
    PlatformAdmin.create!(account: account)
  end
end
