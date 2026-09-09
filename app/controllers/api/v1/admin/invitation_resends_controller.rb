module Api
  module V1
    module Admin
      class InvitationResendsController < BaseController
        rescue_from ActiveRecord::RecordInvalid do
          render_unprocessable('Invitation cannot be resent')
        end
        rescue_from HouseholdInvitations::Resend::FreshAuthenticationRequired do
          render_api_error(code: 'fresh_privileged_action_required', message: 'Fresh authentication is required',
                           status: :forbidden)
        end
        rescue_from HouseholdInvitations::Resend::DeliveryError do
          render_api_error(code: 'invitation_delivery_unavailable', message: 'Invitation delivery is temporarily unavailable',
                           status: :service_unavailable)
        end

        def create
          HouseholdInvitations::Resend.new(invitation: invitation, authorization: pundit_user,
                                           credential: current_api_session, request: request).call
          render json: { data: { invitation_id: invitation.id.to_s, expires_at: invitation.expires_at.iso8601,
                                 delivery_status: 'queued' } }
        end

        private

        def with_api_idempotency(&)
          response.set_header('Cache-Control', 'no-store')
          require_household_manager
          require_fresh_privileged_action unless performed?
          return if performed?

          authorize invitation, :resend?
          super
        end

        def invitation
          @invitation ||= current_household.household_invitations.find(params.expect(:id))
        end
      end
    end
  end
end
