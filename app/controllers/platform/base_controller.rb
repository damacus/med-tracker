# frozen_string_literal: true

module Platform
  class BaseController < ApplicationController
    include HostedPrivilegedActionMfa

    before_action :require_platform_admin

    private

    def pundit_user
      AuthorizationContext.new(account: current_account, household: nil, membership: nil)
    end

    def require_platform_admin
      return if current_account&.platform_admin&.active?

      user_not_authorized
    end
  end
end
