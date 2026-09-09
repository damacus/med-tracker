module Api
  module V1
    class InvitationAcceptancesController < BaseController
      rescue_from HouseholdInvitations::Accept::Unavailable do
        render_api_error(code: 'invitation_unavailable', message: 'Invitation is unavailable',
                         status: :unprocessable_content)
      end

      def create
        membership = HouseholdInvitations::Accept.new(account: current_account, person: current_user.person,
                                                      token: params.expect(:token), request: request).call
        render json: { data: { household_id: membership.household_id.to_s, membership_id: membership.id.to_s,
                               person_id: membership.person_id&.to_s, role: membership.role } }
      end

      private

      def with_api_idempotency(&action)
        response.set_header('Cache-Control', 'no-store')
        return render_forbidden unless current_api_session.is_a?(ApiSession)

        action.call
      end
    end
  end
end
