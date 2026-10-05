# frozen_string_literal: true

class RodauthController < ApplicationController
  # Skip authentication for Rodauth routes to prevent redirect loops
  allow_unauthenticated_access
  skip_after_action :verify_pundit_authorization
  before_action :preload_shell_membership, if: :authenticated?

  # Used by Rodauth for rendering views, CSRF protection, running any
  # registered action callbacks and rescue handlers, instrumentation etc.

  # Controller callbacks and rescue handlers will run around Rodauth endpoints.
  # before_action :verify_captcha, only: :login, if: -> { request.post? }
  # rescue_from("SomeError") { |exception| ... }

  # Layout can be changed for all Rodauth pages or only certain pages.
  # layout "authentication"
  # layout -> do
  #   case rodauth.current_route
  #   when :login, :create_account, :verify_account, :verify_account_resend,
  #        :reset_password, :reset_password_request
  #     "authentication"
  #   else
  #     "application"
  #   end
  # end

  private

  def preload_shell_membership
    @shell_membership = TenantContext.with(
      account: current_account,
      household: nil,
      request_id: request.request_id
    ) do
      current_account.first_active_household_membership
    end
    @default_household_for_urls = @shell_membership&.household
  end

  def shell_membership
    @shell_membership || super
  end
end
