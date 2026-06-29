# frozen_string_literal: true

module Admin
  class BaseController < ApplicationController
    include HostedPrivilegedActionMfa

    before_action :require_hosted_privileged_action_mfa, if: :household_admin_mfa_gate_applies?

    private

    def household_admin_mfa_gate_applies?
      current_membership&.owner? || current_membership&.administrator?
    end
  end
end
