# frozen_string_literal: true

class WebauthnController < ApplicationController
  before_action :authenticate_account!
  before_action :set_webauthn_service

  # Show registration options for new passkey
  def registration_options
    options = @webauthn_service.registration_options
    render json: options
  end

  # Register a new passkey
  def register
    credential = params[:credential]
    nickname = params[:nickname]

    begin
      @webauthn_service.register_credential(credential, nickname)
      render json: { success: true, message: 'Passkey registered successfully' }
    rescue WebAuthnService::Error => e
      render json: { success: false, error: e.message }, status: :unprocessable_content
    end
  end

  # Show authentication options (for login page)
  def authentication_options
    # For discoverable credentials, we don't need to specify allow_credentials
    options = WebAuthn::Credential.options_for_get(
      userVerification: 'required',
      timeout: 120_000 # 2 minutes
    )

    # Store the challenge in the session
    session[:webauthn_challenge] = options.challenge

    render json: options
  end

  # Authenticate with a passkey (login)
  def authenticate
    credential = params[:credential]
    params[:email] # Optional email hint for faster lookup

    begin
      # Try to find the account by credential ID first
      webauthn_credential = WebAuthn::Credential.from_get(credential)
      webauthn_key = AccountWebauthnKey.find_by(webauthn_id: webauthn_credential.id)

      raise WebAuthnService::Error, 'Passkey not found' unless webauthn_key

      account = webauthn_key.account

      # Create service and authenticate
      service = WebAuthnService.new(account)
      service.instance_variable_set(:@account, account)

      # Use the stored challenge from session
      stored_challenge = session[:webauthn_challenge]
      raise WebAuthnService::Error, 'Invalid or expired challenge' unless stored_challenge

      # Verify the credential
      webauthn_credential.verify(
        stored_challenge,
        public_key: webauthn_key.public_key,
        sign_count: webauthn_key.sign_count,
        user_verification: true,
        rp: { name: 'MedTracker', id: Rails.env.production? ? 'medtracker.com' : 'localhost' },
        origin: request_origin
      )

      # Check for cloned credentials
      if webauthn_key.sign_count > webauthn_credential.sign_count
        Rails.logger.warn "Potential cloned passkey detected for account #{account.id}"
      end

      # Update the credential usage
      webauthn_key.update!(
        sign_count: webauthn_credential.sign_count,
        last_use: Time.current
      )

      # Clear the challenge
      session.delete(:webauthn_challenge)

      # Log in the user
      session[:account_id] = account.id

      render json: { success: true, redirect_to: dashboard_path }
    rescue WebAuthnService::Error => e
      render json: { success: false, error: e.message }, status: :unprocessable_content
    rescue WebAuthn::VerificationError => e
      render json: { success: false, error: "Authentication failed: #{e.message}" }, status: :unprocessable_content
    end
  end

  # Remove a passkey
  def remove
    key_id = params[:id]

    begin
      @webauthn_service.remove_credential(key_id)
      redirect_to settings_security_path, notice: t('webauthn.passkey_removed')
    rescue WebAuthnService::Error => e
      redirect_to settings_security_path, alert: e.message
    end
  end

  private

  def set_webauthn_service
    @webauthn_service = WebAuthnService.new(current_account)
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
