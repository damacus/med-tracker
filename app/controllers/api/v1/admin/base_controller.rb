# frozen_string_literal: true

module Api
  module V1
    module Admin
      class BaseController < Api::V1::BaseController
        before_action :require_household_manager

        private

        def require_household_manager
          return if current_membership&.owner? || current_membership&.administrator?

          render_forbidden
        end

        def require_fresh_privileged_action
          return if Api::FreshPrivilegedAction.new(credential: current_api_session).satisfied?

          render_api_error(
            code: 'fresh_privileged_action_required',
            message: 'Fresh MFA or OIDC MFA proof is required for this action.',
            status: :forbidden
          )
        end

        def audit_admin_action!(event_type:, target:, outcome:)
          Audit::Event.record!(
            household: current_household,
            actor_account: current_account,
            actor_membership: current_membership,
            event_type: event_type,
            request: request,
            metadata: {
              target_type: target.class.name,
              target_id: target.id,
              outcome: outcome
            }
          )
        end

        def audit_context
          {
            whodunnit: current_user&.id,
            ip: request.remote_ip,
            request_id: request.request_id,
            household_id: current_household.id,
            actor_membership_id: current_membership.id
          }
        end
      end
    end
  end
end
