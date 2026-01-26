# frozen_string_literal: true

# Service for managing WebAuthn/passkey operations
class WebAuthnService
  # rubocop:disable Metrics/MethodLength, Metrics/AbcSize
  class Error < StandardError; end

  def initialize(account)
    @account = account
  end

  # Prepare options for passkey registration
  def registration_options
    credential_options = WebAuthn::Credential.options_for_create(
      rp: relying_party,
      user: user,
      attestation: 'direct',
      authenticator_selection: {
        user_verification: 'required',
        resident_key: 'preferred',
        authenticator_attachment: nil # Allow both platform and cross-platform
      },
      timeout: 120_000 # 2 minutes
    )

    # Store the challenge in the session for verification
    Rails.cache.write("webauthn_challenge_#{account.id}", credential_options.challenge, expires_in: 5.minutes)

    credential_options
  end

  # Register a new passkey
  def register_credential(webauthn_credential, nickname = nil)
    # Verify the challenge
    stored_challenge = Rails.cache.read("webauthn_challenge_#{account.id}")
    raise Error, 'Invalid or expired challenge' unless stored_challenge

    # Create the credential
    webauthn_credential = WebAuthn::Credential.from_create(webauthn_credential)

    # Verify the credential
    begin
      webauthn_credential.verify(
        stored_challenge,
        rp: relying_party,
        origin: request_origin,
        attestation: 'direct'
      )
    rescue WebAuthn::VerificationError => e
      raise Error, "Credential verification failed: #{e.message}"
    end

    # Store the credential
    account_webauthn_user_id = AccountWebauthnUserId.create!(
      account_id: account.id,
      webauthn_id: webauthn_credential.id
    )

    AccountWebauthnKey.create!(
      account_id: account.id,
      webauthn_id: webauthn_credential.id,
      public_key: webauthn_credential.public_key,
      sign_count: webauthn_credential.sign_count,
      nickname: nickname || "Passkey #{account.account_webauthn_keys.count + 1}"
    )

    # Clear the challenge
    Rails.cache.delete("webauthn_challenge_#{account.id}")

    account_webauthn_user_id
  end

  # Prepare options for passkey authentication
  def authentication_options
    # Get all credential IDs for the account
    credential_ids = account.account_webauthn_keys.pluck(:webauthn_id)

    # If no credentials, allow discoverable credentials
    allow_credentials = credential_ids.map { |id| { id: id, type: 'public-key' } }

    credential_options = WebAuthn::Credential.options_for_get(
      allow_credentials: allow_credentials,
      user_verification: 'required',
      timeout: 120_000 # 2 minutes
    )

    # Store the challenge in the session
    Rails.cache.write("webauthn_auth_challenge_#{account.id}", credential_options.challenge, expires_in: 5.minutes)

    credential_options
  end

  # Authenticate with a passkey
  # rubocop:disable Metrics/CyclomaticComplexity, Metrics/PerceivedComplexity
  def authenticate_credential(webauthn_credential, account_lookup = nil)
    # Get the stored challenge
    stored_challenge = if account_lookup
                         Rails.cache.read("webauthn_auth_challenge_#{account_lookup.id}")
                       else
                         Rails.cache.read("webauthn_auth_challenge_#{account.id}")
                       end

    raise Error, 'Invalid or expired challenge' unless stored_challenge

    # Create and verify the credential
    webauthn_credential = WebAuthn::Credential.from_get(webauthn_credential)

    # Find the stored credential
    stored_key = if account_lookup
                   AccountWebauthnKey.find_by(webauthn_id: webauthn_credential.id)
                 else
                   account.account_webauthn_keys.find_by(webauthn_id: webauthn_credential.id)
                 end

    raise Error, 'Credential not found' unless stored_key

    # Get the public key
    public_key = stored_key.public_key
    sign_count = stored_key.sign_count

    # Verify the authentication
    begin
      webauthn_credential.verify(
        stored_challenge,
        public_key: public_key,
        sign_count: sign_count,
        user_verification: true,
        rp: relying_party,
        origin: request_origin
      )
    rescue WebAuthn::VerificationError => e
      raise Error, "Authentication failed: #{e.message}"
    end

    # Check for cloned credentials
    if sign_count > webauthn_credential.sign_count
      Rails.logger.warn "Potential cloned passkey detected for account #{stored_key.account_id}: " \
                        "stored count #{sign_count} > current count #{webauthn_credential.sign_count}"
    end

    # In production, you might want to:
    # - Send security alert
    # - Lock account
    # - Require password re-auth

    # Update the credential usage
    stored_key.update!(
      sign_count: webauthn_credential.sign_count,
      last_use: Time.current
    )

    # Clear the challenge
    if account_lookup
      Rails.cache.delete("webauthn_auth_challenge_#{account_lookup.id}")
    else
      Rails.cache.delete("webauthn_auth_challenge_#{account.id}")
    end

    stored_key.account
  end
  # rubocop:enable Metrics/MethodLength, Metrics/AbcSize, Metrics/CyclomaticComplexity, Metrics/PerceivedComplexity

  # Remove a passkey
  def remove_credential(webauthn_key_id)
    webauthn_key = account.account_webauthn_keys.find(webauthn_key_id)

    # Also remove the user ID record
    AccountWebauthnUserId.where(account_id: account.id, webauthn_id: webauthn_key.webauthn_id).delete_all

    webauthn_key.destroy!
  end

  # List all passkeys for the account
  def credentials
    account.account_webauthn_keys.order(created_at: :desc)
  end

  private

  attr_reader :account

  def relying_party
    @relying_party ||= {
      name: 'MedTracker',
      id: Rails.env.production? ? 'medtracker.com' : 'localhost'
    }
  end

  def user
    {
      id: account.id.to_s,
      name: account.person&.name || account.email,
      display_name: account.person&.name || account.email
    }
  end

  def request_origin
    if Rails.env.development?
      'http://localhost:3000'
    elsif Rails.env.test?
      'https://localhost:3001'
    else
      "https://#{Rails.application.credentials.dig(:webauthn, :host) || 'medtracker.com'}"
    end
  end
end
