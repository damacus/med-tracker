# frozen_string_literal: true

module Admin
  class BaseController < ApplicationController
    include HostedPrivilegedActionMfa

    before_action :require_hosted_privileged_action_mfa, if: :household_admin_mfa_gate_applies?

    private

    def household_admin_mfa_gate_applies?
      current_membership&.owner? ||
        current_membership&.administrator? ||
        support_access_session_mfa_gate_applies?
    end

    def support_access_session_mfa_gate_applies?
      Current.support_access_session&.active? &&
        Current.support_access_session.household_id == Current.household&.id
    end
  end
end
