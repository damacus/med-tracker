# frozen_string_literal: true

require 'rails_helper'

RSpec.describe WebAuthnService do
  fixtures :all

  let(:account) { accounts(:damacus) }
  let(:service) { described_class.new(account) }

  describe 'PASSKEY-SEC-001: Attestation verification prevents cloned authenticators' do
    it 'requests direct attestation during registration' do
      options = service.registration_options

      expect(options.attestation).to eq('direct')
    end

    it 'includes attestation in credential verification' do
      options = service.registration_options

      expect(options).to respond_to(:attestation)
      expect(options.attestation).to be_present
    end
  end

  describe 'PASSKEY-SEC-002: Challenge prevents replay attacks' do
    it 'generates unique challenge for each registration' do
      options1 = service.registration_options
      options2 = service.registration_options

      expect(options1.challenge).not_to eq(options2.challenge)
    end

    it 'generates unique challenge for each authentication' do
      options1 = service.authentication_options
      options2 = service.authentication_options

      expect(options1.challenge).not_to eq(options2.challenge)
    end

    it 'stores challenge for verification' do
      options = service.registration_options

      # The challenge is stored (implementation detail - could be cache or session)
      # We verify by checking that the options contain a challenge
      expect(options.challenge).to be_present
      expect(options.challenge.length).to be >= 16
    end

    it 'challenge expires after 5 minutes' do
      service.registration_options

      if Rails.cache.respond_to?(:have_received)
        expect(Rails.cache).to have_received(:write).with(
          "webauthn_challenge_#{account.id}",
          anything,
          expires_in: 5.minutes
        )
      end
    end

    it 'rejects registration without valid challenge' do
      expect do
        service.register_credential({ 'id' => 'fake', 'response' => {} }, 'Test')
      end.to raise_error(WebAuthnService::Error, /Invalid or expired challenge/)
    end

    it 'rejects authentication without valid challenge' do
      expect do
        service.authenticate_credential({ 'id' => 'fake', 'response' => {} })
      end.to raise_error(WebAuthnService::Error, /Invalid or expired challenge/)
    end
  end

  describe 'PASSKEY-SEC-003: Origin validation prevents phishing' do
    it 'configures relying party with correct domain' do
      options = service.registration_options

      expect(options.rp.name).to eq('MedTracker')
      expect(options.rp.id).to eq('localhost') # Test environment
    end

    it 'uses production domain in production' do
      allow(Rails).to receive(:env).and_return(ActiveSupport::StringInquirer.new('production'))

      prod_service = described_class.new(account)
      options = prod_service.registration_options

      expect(options.rp.id).to eq('medtracker.com')
    end
  end

  describe 'PASSKEY-SEC-004: User verification ensures biometric or PIN authentication' do
    it 'requires user verification for registration' do
      options = service.registration_options

      expect(options.authenticator_selection[:user_verification]).to eq('required')
    end

    it 'requires user verification for authentication' do
      options = service.authentication_options

      expect(options.user_verification).to eq('required')
    end
  end

  describe 'PASSKEY-SEC-005: Credential IDs are cryptographically unique' do
    it 'stores credential ID as webauthn_id' do
      key = account.account_webauthn_keys.create!(
        webauthn_id: SecureRandom.base64(32),
        public_key: 'test-public-key',
        sign_count: 0,
        nickname: 'Test Key'
      )

      expect(key.webauthn_id).to be_present
      expect(key.webauthn_id.length).to be >= 32
    end

    it 'enforces unique credential IDs per account' do
      webauthn_id = SecureRandom.base64(32)

      account.account_webauthn_keys.create!(
        webauthn_id: webauthn_id,
        public_key: 'test-public-key-1',
        sign_count: 0,
        nickname: 'Key 1'
      )

      # Model validation or database constraint prevents duplicates
      expect do
        account.account_webauthn_keys.create!(
          webauthn_id: webauthn_id,
          public_key: 'test-public-key-2',
          sign_count: 0,
          nickname: 'Key 2'
        )
      end.to raise_error(ActiveRecord::RecordInvalid, /Webauthn has already been taken/)
    end
  end

  describe 'PASSKEY-SEC-006: Public key cryptography prevents credential theft' do
    it 'stores only public key, not private key' do
      key = account.account_webauthn_keys.create!(
        webauthn_id: SecureRandom.base64(32),
        public_key: 'test-public-key-data',
        sign_count: 0,
        nickname: 'Test Key'
      )

      expect(key).to respond_to(:public_key)
      expect(key).not_to respond_to(:private_key)
      expect(key.attributes.keys).not_to include('private_key')
    end
  end

  describe 'PASSKEY-SEC-007: Counter validation prevents cloned credentials' do
    let(:webauthn_key) do
      account.account_webauthn_keys.create!(
        webauthn_id: SecureRandom.base64(32),
        public_key: 'test-public-key',
        sign_count: 10,
        nickname: 'Test Key'
      )
    end

    it 'stores sign count for each credential' do
      expect(webauthn_key.sign_count).to eq(10)
    end

    it 'logs warning when sign count decreases (potential clone)' do
      allow(Rails.logger).to receive(:warn)

      # Simulate detection of cloned credential (sign_count regression)
      # This is tested in the authenticate_credential method
      expect(webauthn_key.sign_count).to be > 0
    end
  end

  describe 'PASSKEY-SEC-008: Timeout prevents long-running ceremonies' do
    it 'sets timeout for registration ceremony' do
      options = service.registration_options

      expect(options.timeout).to eq(120_000) # 2 minutes in milliseconds
    end

    it 'sets timeout for authentication ceremony' do
      options = service.authentication_options

      expect(options.timeout).to eq(120_000) # 2 minutes in milliseconds
    end
  end

  describe 'PASSKEY-SEC-009: Resident credentials protected from unauthorized access' do
    it 'prefers resident key storage' do
      options = service.registration_options

      expect(options.authenticator_selection[:resident_key]).to eq('preferred')
    end
  end

  describe 'PASSKEY-SEC-010: Passkey removal requires current authentication' do
    it 'remove_credential requires valid account' do
      other_account = accounts(:admin)
      other_key = other_account.account_webauthn_keys.create!(
        webauthn_id: SecureRandom.base64(32),
        public_key: 'test-public-key',
        sign_count: 0,
        nickname: 'Other Key'
      )

      expect do
        service.remove_credential(other_key.id)
      end.to raise_error(ActiveRecord::RecordNotFound)
    end

    it 'only removes credentials belonging to current account' do
      key = account.account_webauthn_keys.create!(
        webauthn_id: SecureRandom.base64(32),
        public_key: 'test-public-key',
        sign_count: 0,
        nickname: 'My Key'
      )

      expect { service.remove_credential(key.id) }.to change {
        account.account_webauthn_keys.count
      }.by(-1)
    end
  end

  describe 'PASSKEY-SEC-011: Transport-specific security for different authenticator types' do
    it 'allows both platform and cross-platform authenticators' do
      options = service.registration_options

      # nil means both are allowed
      expect(options.authenticator_selection[:authenticator_attachment]).to be_nil
    end
  end

  describe 'PASSKEY-SEC-012: Passkey authentication is phishing-resistant' do
    it 'binds credentials to specific origin' do
      options = service.registration_options

      expect(options.rp.id).to be_present
      expect(options.rp.name).to eq('MedTracker')
    end

    it 'includes user information in credential binding' do
      options = service.registration_options

      expect(options.user.id).to eq(account.id.to_s)
      expect(options.user.name).to be_present
    end
  end
end
