# frozen_string_literal: true

require 'rails_helper'

RSpec.describe ApiAuthState do
  fixtures :accounts

  describe '.web_session_mfa_satisfied?' do
    let(:account) { accounts(:damacus) }

    before do
      AccountOtpKey.where(id: account.id).delete_all
      account.account_webauthn_keys.destroy_all
    end

    it 'does not require MFA session evidence when no MFA is configured' do
      expect(described_class.web_session_mfa_satisfied?({}, account)).to be(true)
    end

    it 'accepts a TOTP-authenticated Rodauth web session for an MFA-configured account' do
      AccountOtpKey.create!(id: account.id, key: 'test_otp_key_secret')

      expect(described_class.web_session_mfa_satisfied?({ authenticated_by: %w[password totp] }, account)).to be(true)
    end

    it 'accepts an OIDC session when the upstream provider performed MFA' do
      AccountOtpKey.create!(id: account.id, key: 'test_otp_key_secret')

      expect(described_class.web_session_mfa_satisfied?({ oidc_mfa_verified: true }, account)).to be(true)
    end

    it 'rejects a password-only web session for an MFA-configured account' do
      AccountOtpKey.create!(id: account.id, key: 'test_otp_key_secret')

      expect(described_class.web_session_mfa_satisfied?({ authenticated_by: ['password'] }, account)).to be(false)
    end
  end
end
