# frozen_string_literal: true

module Api
  module V1
    module Admin
      class InvitationsController < BaseController
        before_action :require_fresh_privileged_action, only: %i[create destroy]

        def index
          invitations = current_household.household_invitations.order(created_at: :desc).limit(100)
          render json: { data: invitations.map { |invitation| invitation_payload(invitation) } }
        end

        def create
          invitation = current_household.household_invitations.new(invitation_params)
          invitation.invited_by_membership = current_membership

          return render_validation_errors(invitation) unless invitation.save

          audit_admin_action!(event_type: 'api/admin/invitation/created', target: invitation, outcome: 'success')
          render json: { data: invitation_payload(invitation) }, status: :created
        end

        def destroy
          invitation = current_household.household_invitations.find(params.expect(:id))
          invitation.update!(revoked_at: Time.current)
          audit_admin_action!(event_type: 'api/admin/invitation/revoked', target: invitation, outcome: 'success')
          head :no_content
        end

        private

        def invitation_params
          params.expect(household_invitation: %i[email membership_role])
        end

        def invitation_payload(invitation)
          {
            id: invitation.id,
            email: invitation.email,
            membership_role: invitation.membership_role,
            pending: invitation.pending?,
            accepted_at: invitation.accepted_at&.iso8601,
            revoked_at: invitation.revoked_at&.iso8601,
            expires_at: invitation.expires_at.iso8601
          }
        end
      end
    end
  end
end
