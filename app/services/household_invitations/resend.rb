module HouseholdInvitations
  class Resend
    class FreshAuthenticationRequired < StandardError
    end

    class DeliveryError < StandardError
    end

    def initialize(invitation:, authorization:, credential:, request: nil)
      @invitation = invitation
      @authorization = authorization
      @credential = credential
      @request = request
    end

    def call
      Households::LifecycleCutoffLock.with(household_id: @invitation.household_id) do
        @invitation.household.with_lock do
          @invitation.with_lock do
            authorize_current_actor!
            @invitation.resend!
            record_resend!
            enqueue_mail!
          end
        end
      end
      @invitation
    end

    private

    def authorize_current_actor!
      membership = @authorization.membership.reload
      Pundit.authorize(@authorization.with(membership: membership), @invitation, :resend?)
      authorize_household!

      return if valid_session?(membership) && Api::FreshPrivilegedAction.new(credential: @credential).satisfied?

      raise FreshAuthenticationRequired
    end

    def authorize_household!
      raise Pundit::NotAuthorizedError unless @invitation.household_id == @authorization.household.id
      return if @invitation.household.operational? && @authorization.account.reload.verified?

      raise Pundit::NotAuthorizedError
    end

    def valid_session?(membership)
      return false unless @credential.is_a?(ApiSession)

      @credential.reload
      @credential.revoked_at.nil? && @credential.access_expires_at.future? && @credential.active_for_membership? &&
        @credential.household_membership_id == membership.id && @credential.account_id == membership.account_id
    end

    def record_resend!
      Audit::Event.record!(household: @invitation.household, actor_account: @authorization.account,
                           actor_membership: @authorization.membership, request: @request,
                           event_type: 'api/admin/invitation/resent',
                           metadata: { target_type: 'HouseholdInvitation', target_id: @invitation.id,
                                       outcome: 'success' })
    end

    def enqueue_mail!
      delivery = InvitationMailer.with(invitation: @invitation, token: @invitation.plain_token).invite.deliver_later
      raise DeliveryError unless delivery
    rescue StandardError => e
      Observability::DiagnosticEvent.failure(component: :invitation_delivery, error: e)
      raise DeliveryError
    end
  end
end
