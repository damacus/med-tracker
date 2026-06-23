# frozen_string_literal: true

module Authentication
  extend ActiveSupport::Concern

  # Household managers should have 2FA enabled for security

  included do
    before_action :require_authentication
    helper_method :authenticated?, :current_user, :current_account, :should_setup_two_factor?
  end

  class_methods do
    def allow_unauthenticated_access(**)
      skip_before_action(:require_authentication, **)
    end
  end

  private

  # Get current user via Rodauth account
  def current_user
    return nil unless current_account

    @current_user ||= current_account.person&.user ||
                      User.joins(:person).find_by(people: { account_id: current_account.id })
  end

  # Get current Rodauth account
  def current_account
    @current_account ||= Account.find_by(id: rodauth.session_value) if rodauth.logged_in?
  end

  def authenticated?
    rodauth.logged_in? && active_current_user?
  end

  def require_authentication
    rodauth.require_login
    return unless rodauth.logged_in?
    return if active_current_user?

    reset_inactive_session
  end

  def request_authentication
    session[:return_to_after_authenticating] = request.url
    redirect_to rodauth.login_path
  end

  def active_current_user?
    current_user&.active?
  end

  def reset_inactive_session
    reset_session
    @current_account = nil
    @current_user = nil

    redirect_to rodauth.login_path, alert: inactive_account_message, status: :see_other
  end

  def inactive_account_message
    t('authentication.inactive_account', default: 'Your account has been deactivated. Please contact an administrator.')
  end

  def after_authentication_url
    session.delete(:return_to_after_authenticating) || root_url
  end

  # Soft enforcement: Show flash notice to privileged users without 2FA
  # Does NOT block access - just reminds them to set it up
  def check_two_factor_setup
    return unless should_setup_two_factor?
    return if flash[:warning].present?

    flash.now[:warning] = I18n.t('authentication.two_factor_required')
  end

  # Check if current account should have 2FA set up
  def should_setup_two_factor?
    return false unless rodauth.logged_in?
    return false unless current_user
    return false if two_factor_configured?
    return false if oidc_authenticated?

    household_manager_requires_two_factor?
  end

  def household_manager_requires_two_factor?
    Current.membership&.owner? || Current.membership&.administrator? || false
  end

  def oidc_authenticated?
    session[:oidc_mfa_verified] == true
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
