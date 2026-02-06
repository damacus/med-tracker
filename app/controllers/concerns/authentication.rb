# frozen_string_literal: true

module Authentication
  extend ActiveSupport::Concern

  # Roles that should have 2FA enabled for security
  ROLES_REQUIRING_2FA = %w[administrator doctor nurse].freeze

  included do
    before_action :require_authentication
    before_action :check_two_factor_setup
    helper_method :authenticated?, :current_user, :current_account, :should_setup_two_factor?
  end

  class_methods do
    def allow_unauthenticated_access(**)
      skip_before_action(:require_authentication, **)
      skip_before_action(:check_two_factor_setup, **)
    end

    def skip_two_factor_check(**)
      skip_before_action(:check_two_factor_setup, **)
    end
  end

  private

  # Get current user via Rodauth account
  def current_user
    @current_user ||= current_account&.person&.user
  end

  # Get current Rodauth account
  def current_account
    @current_account ||= Account.find_by(id: rodauth.session_value) if rodauth.logged_in?
  end

  def authenticated?
    rodauth.authenticated?
  end

  def require_authentication
    rodauth.require_authentication
  end

  def request_authentication
    session[:return_to_after_authenticating] = request.url
    redirect_to rodauth.login_path
  end

  def after_authentication_url
    session.delete(:return_to_after_authenticating) || root_url
  end

  # Soft enforcement: Show flash notice to privileged users without 2FA
  # Does NOT block access - just reminds them to set it up
  def check_two_factor_setup
    return unless should_setup_two_factor?
    return if request.path.start_with?('/otp-setup', '/webauthn-setup', '/recovery-codes', '/multifactor')
    return if flash.any? # Don't overwrite existing flash messages

    flash.now[:warning] = I18n.t('authentication.two_factor_required')
  end

  # Check if current user should have 2FA set up
  def should_setup_two_factor?
    return false unless rodauth.logged_in?
    return false unless current_user
    return false if two_factor_configured?

    ROLES_REQUIRING_2FA.include?(current_user.role)
  end

  # Check if user has any 2FA method configured
  def two_factor_configured?
    return false unless current_account

    # Check for TOTP
    return true if AccountOtpKey.exists?(id: current_account.id)

    # Check for WebAuthn/Passkeys
    return true if current_account.account_webauthn_keys.exists?

    false
  end
end
