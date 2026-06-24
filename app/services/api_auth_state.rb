# frozen_string_literal: true

class ApiAuthState
  MFA_METHODS = %w[totp webauthn sms_code recovery_code otp].freeze

  class << self
    def password_authenticated?(account, password)
      return false if account.blank? || password.blank?
      return false unless account.verified?

      BCrypt::Password.new(account.password_hash).is_password?(password)
    rescue BCrypt::Errors::InvalidHash
      false
    end

    def locked_out?(account)
      return false if account.blank?

      AccountLockout.active.exists?(account_id: account.id)
    end

    def mfa_configured?(account)
      return false if account.blank?

      AccountOtpKey.exists?(id: account.id) || account.account_webauthn_keys.exists?
    end

    def web_session_mfa_satisfied?(session, account)
      return true unless mfa_configured?(account)
      return true if session[:oidc_mfa_verified] == true || session['oidc_mfa_verified'] == true

      authenticated_by = Array(session[:authenticated_by]) | Array(session['authenticated_by'])
      authenticated_by.intersect?(MFA_METHODS)
    end
  end
end
