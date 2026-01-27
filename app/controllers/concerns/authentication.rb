# frozen_string_literal: true

module Authentication
  extend ActiveSupport::Concern

  included do
    before_action :require_authentication
    helper_method :authenticated?, :current_user, :current_account
  end

  class_methods do
    def allow_unauthenticated_access(**)
      skip_before_action(:require_authentication, **)
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
end
