# frozen_string_literal: true

require 'rails_helper'

RSpec.describe ApiAuthState do
  fixtures :accounts

  describe '.password_authenticated?' do
    let(:account) { accounts(:damacus) }

    it 'accepts the password for a verified account' do
      expect(described_class.password_authenticated?(account, 'password')).to be(true)
    end

    it 'rejects an incorrect password for a verified account' do
      expect(described_class.password_authenticated?(account, 'wrong-password')).to be(false)
    end

    it 'rejects blank accounts and passwords' do
      expect(described_class.password_authenticated?(nil, 'password')).to be(false)
      expect(described_class.password_authenticated?(account, nil)).to be(false)
    end

    it 'rejects unverified accounts' do
      account.update!(status: :unverified)

      expect(described_class.password_authenticated?(account, 'password')).to be(false)
    end

    it 'rejects invalid password hashes' do
      account.update!(password_hash: 'not-a-bcrypt-hash')

      expect(described_class.password_authenticated?(account, 'password')).to be(false)
    end
  end

  describe '.locked_out?' do
    let(:account) { accounts(:damacus) }

    it 'returns false without an account' do
      expect(described_class.locked_out?(nil)).to be(false)
    end

    it 'returns false when the account has no active lockout' do
      expect(described_class.locked_out?(account)).to be(false)
    end

    it 'detects active lockouts and ignores expired lockouts' do
      lockout = AccountLockout.create!(account: account, key: 'lockout-key', deadline: 5.minutes.from_now)

      expect(described_class.locked_out?(account)).to be(true)

      lockout.update!(deadline: 5.minutes.ago)

      expect(described_class.locked_out?(account)).to be(false)
    end
  end

  describe '.mfa_configured?' do
    let(:account) { accounts(:damacus) }

    before do
      AccountOtpKey.where(id: account.id).delete_all
      account.account_webauthn_keys.destroy_all
    end

    it 'returns false without an account' do
      expect(described_class.mfa_configured?(nil)).to be(false)
    end

    it 'returns false when the account has no configured MFA method' do
      expect(described_class.mfa_configured?(account)).to be(false)
    end

    it 'detects TOTP configuration' do
      AccountOtpKey.create!(id: account.id, key: 'test_otp_key_secret')

      expect(described_class.mfa_configured?(account)).to be(true)
    end

    it 'detects WebAuthn passkey configuration' do
      account.account_webauthn_keys.create!(webauthn_id: 'passkey-id', public_key: 'public-key', sign_count: 0)

      expect(described_class.mfa_configured?(account)).to be(true)
    end
  end

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

    it 'accepts string-keyed OIDC MFA proof for configured accounts' do
      AccountOtpKey.create!(id: account.id, key: 'test_otp_key_secret')

      expect(described_class.web_session_mfa_satisfied?({ 'oidc_mfa_verified' => true }, account)).to be(true)
    end

    it 'accepts string-keyed local MFA method proof for configured accounts' do
      AccountOtpKey.create!(id: account.id, key: 'test_otp_key_secret')

      session = { 'authenticated_by' => %w[password otp] }

      expect(described_class.web_session_mfa_satisfied?(session, account)).to be(true)
    end

    it 'rejects a password-only web session for an MFA-configured account' do
      AccountOtpKey.create!(id: account.id, key: 'test_otp_key_secret')

      expect(described_class.web_session_mfa_satisfied?({ authenticated_by: ['password'] }, account)).to be(false)
    end
  end

  describe '.web_session_oidc_mfa_verified?' do
    it 'accepts symbol-keyed and string-keyed OIDC MFA proof' do
      expect(described_class.web_session_oidc_mfa_verified?({ oidc_mfa_verified: true })).to be(true)
      expect(described_class.web_session_oidc_mfa_verified?({ 'oidc_mfa_verified' => true })).to be(true)
    end

    it 'rejects missing or false OIDC MFA proof' do
      expect(described_class.web_session_oidc_mfa_verified?({})).to be(false)
      expect(described_class.web_session_oidc_mfa_verified?({ oidc_mfa_verified: false })).to be(false)
      expect(described_class.web_session_oidc_mfa_verified?({ oidc_mfa_verified: 'true' })).to be(false)
    end
  end

  describe '.web_session_mfa_method_present?' do
    it 'accepts symbol-keyed and string-keyed MFA authentication methods' do
      expect(described_class.web_session_mfa_method_present?({ authenticated_by: %w[password webauthn] })).to be(true)
      expect(described_class.web_session_mfa_method_present?({ 'authenticated_by' => %w[password otp] })).to be(true)
    end

    it 'rejects password-only or missing authentication method evidence' do
      expect(described_class.web_session_mfa_method_present?({ authenticated_by: ['password'] })).to be(false)
      expect(described_class.web_session_mfa_method_present?({})).to be(false)
    end

    it 'combines symbol-keyed and string-keyed authentication method evidence' do
      session = { authenticated_by: ['password'], 'authenticated_by' => ['recovery_code'] }

      expect(described_class.web_session_mfa_method_present?(session)).to be(true)
    end
  end
end
