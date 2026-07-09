# frozen_string_literal: true

module Platform
  class BaseController < ApplicationController
    include HostedPrivilegedActionMfa

    before_action :require_platform_admin
    before_action :require_privileged_action_mfa, if: :platform_write_action?

    private

    def pundit_user
      AuthorizationContext.new(account: current_account, household: nil, membership: nil)
    end

    def require_platform_admin
      return if current_account&.platform_admin&.active?

      user_not_authorized
    end

    def platform_write_action?
      !request.get? && !request.head?
    end
  end
end
