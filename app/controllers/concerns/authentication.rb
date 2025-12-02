# frozen_string_literal: true

module Authentication
  extend ActiveSupport::Concern

  # Roles that require 2FA to be enabled
  ROLES_REQUIRING_2FA = %w[administrator doctor nurse].freeze

  included do
    before_action :require_authentication
    before_action :require_two_factor_setup
    helper_method :authenticated?, :current_user, :current_account
  end

  class_methods do
    def allow_unauthenticated_access(**)
      skip_before_action(:require_authentication, **)
      skip_before_action(:require_two_factor_setup, **)
    end

    def skip_two_factor_requirement(**)
      skip_before_action(:require_two_factor_setup, **)
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
    rodauth.logged_in?
  end

  def require_authentication
    return if rodauth.logged_in?

    request_authentication
  end

  def request_authentication
    session[:return_to_after_authenticating] = request.url
    redirect_to rodauth.login_path
  end

  def after_authentication_url
    session.delete(:return_to_after_authenticating) || root_url
  end

  # Require 2FA setup for roles that need it (administrator, doctor, nurse)
  def require_two_factor_setup
    return unless rodauth.logged_in?
    return unless current_user
    return unless role_requires_two_factor?
    return if two_factor_enabled?

    # Don't redirect if already on 2FA setup page
    return if request.path == rodauth.otp_setup_path

    flash[:alert] = t('authentication.two_factor_required',
                      default: 'Your role requires two-factor authentication. Please set it up to continue.')
    redirect_to rodauth.otp_setup_path
  end

  def role_requires_two_factor?
    ROLES_REQUIRING_2FA.include?(current_user.role)
  end

  def two_factor_enabled?
    rodauth.uses_two_factor_authentication?
  end
end
