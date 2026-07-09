# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Api::FreshPrivilegedAction do
  it 'requires an API session credential' do
    app_token = instance_double(ApiAppToken)

    expect(described_class.new(credential: app_token)).not_to be_satisfied
  end

  it 'requires an OIDC MFA verified session' do
    session = ApiSession.new(oidc_mfa_verified: false, mfa_verified_at: Time.current)

    expect(described_class.new(credential: session)).not_to be_satisfied
  end

  it 'requires an MFA proof timestamp' do
    session = ApiSession.new(oidc_mfa_verified: true, mfa_verified_at: nil)

    expect(described_class.new(credential: session)).not_to be_satisfied
  end

  it 'rejects stale MFA proof timestamps' do
    session = ApiSession.new(oidc_mfa_verified: true, mfa_verified_at: 16.minutes.ago)

    expect(described_class.new(credential: session)).not_to be_satisfied
  end

  it 'accepts fresh OIDC MFA proof timestamps' do
    session = ApiSession.new(oidc_mfa_verified: true, mfa_verified_at: 1.minute.ago)

    expect(described_class.new(credential: session)).to be_satisfied
  end
end
