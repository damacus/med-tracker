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

  # Primary method: get current user via Rodauth account
  def current_user
    @current_user ||= current_account&.person&.user || legacy_current_user
  end

  # Get current Rodauth account
  def current_account
    @current_account ||= Account.find_by(id: rodauth.session_value) if rodauth.logged_in?
  end

  # Legacy support for existing User-based sessions
  def legacy_current_user
    Current.session&.user
  end

  def authenticated?
    rodauth.logged_in? || resume_session
  end

  def require_authentication
    return if rodauth.logged_in?
    return if resume_session

    request_authentication
  end

  def resume_session
    Current.session ||= find_session_by_cookie
  end

  def find_session_by_cookie
    Session.find_by(id: cookies.signed[:session_id]) if cookies.signed[:session_id]
  end

  def request_authentication
    session[:return_to_after_authenticating] = request.url
    redirect_to rodauth.login_path
  end

  def after_authentication_url
    session.delete(:return_to_after_authenticating) || root_url
  end

  # Legacy session management (will be removed in Phase 5)
  def start_new_session_for(user)
    user.sessions.create!(user_agent: request.user_agent, ip_address: request.remote_ip).tap do |session|
      Current.session = session
      cookies.signed.permanent[:session_id] = { value: session.id, httponly: true, same_site: :lax }
    end
  end

  def terminate_session
    Current.session&.destroy
    cookies.delete(:session_id)
  end
end
