# frozen_string_literal: true

module Authentication
  extend ActiveSupport::Concern

  included do
    before_action :require_authentication
    helper_method :authenticated?, :current_account, :current_person
  end

  class_methods do
    def allow_unauthenticated_access(**)
      skip_before_action(:require_authentication, **)
    end
  end

  private

  # Use Rodauth's current account
  def current_account
    @current_account ||= rodauth.rails_account
  end

  # Get the current person associated with the account
  def current_person
    @current_person ||= current_account&.person
  end

  # For backwards compatibility with existing code
  def current_user
    current_person
  end

  def authenticated?
    rodauth.logged_in?
  end

  def require_authentication
    rodauth.require_account
  end

  def request_authentication
    redirect_to rodauth.login_path
  end
end
