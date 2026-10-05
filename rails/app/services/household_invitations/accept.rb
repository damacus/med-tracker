module HouseholdInvitations
  class Accept
    class Unavailable < StandardError
    end

    def initialize(account:, person:, token:, request: nil)
      @account = account
      @person = person
      @token = token
      @request = request
    end

    def call
      raise Unavailable unless @account.reload.verified? && @token.is_a?(String) && @token.present?

      invitation = TokenResolver.call(@token)
      return replay_accepted if invitation.nil?

      raise Unavailable unless identity_matches?(invitation)

      accept_pending(invitation)
    rescue ActiveRecord::RecordInvalid, CareDelegation::Assign::Error
      raise Unavailable
    end

    private

    def accept_pending(invitation)
      household = invitation.household
      Households::LifecycleCutoffLock.with(household_id: household.id) do
        TenantContext.with(account: @account, household: household) do
          household.with_lock do
            invitation.with_lock do
              next current_accepted_membership(invitation) if invitation.accepted?

              validate_pending!(invitation)
              membership = create_membership(invitation)
              invitation.update!(accepted_at: Time.current)
              membership
            end
          end
        end
      end
    end

    def validate_pending!(invitation)
      validate_current_identity!(invitation)
      raise Unavailable unless invitation.household.operational?
      raise Unavailable unless pending_invitation?(invitation)

      validate_inviter!(invitation.invited_by_membership)
      raise Unavailable if invitation.household.household_memberships.exists?(account: @account)
    end

    def pending_invitation?(invitation)
      invitation.accepted_at.nil? && invitation.revoked_at.nil? && invitation.expires_at.future?
    end

    def validate_inviter!(inviter)
      inviter.reload
      raise Unavailable unless inviter.active? && (inviter.owner? || inviter.administrator?)
    end

    def create_membership(invitation)
      attributes = @person.attributes.slice('name', 'date_of_birth', 'person_type', 'has_capacity')
      person = invitation.household.people.create!(attributes.merge(account_id: @account.id))
      CreateMembership.new(invitation: invitation, account: @account, person: person, request: @request).call
    end

    def replay_accepted
      digest = HouseholdInvitation.digest(@token)
      @account.household_memberships.active.includes(:household).find_each do |membership|
        next unless membership.household.operational?

        accepted = find_accepted(membership, digest)
        return accepted if accepted
      end
      raise Unavailable
    end

    def find_accepted(membership, digest)
      TenantContext.with(account: @account, household: membership.household, membership: membership) do
        membership.household.with_lock do
          invitation = membership.household.household_invitations.where(token_digest: digest, revoked_at: nil)
                                 .where.not(accepted_at: nil).find_by(email: @account.email)
          current_accepted_membership(invitation) if invitation
        end
      end
    end

    def identity_matches?(invitation)
      invitation.email == @account.email.to_s.strip.downcase &&
        invitation.token_digest == HouseholdInvitation.digest(@token)
    end

    def validate_current_identity!(invitation)
      raise Unavailable unless @account.reload.verified? && identity_matches?(invitation)
    end

    def current_accepted_membership(invitation)
      validate_current_identity!(invitation)
      raise Unavailable unless invitation.revoked_at.nil?
      raise Unavailable unless invitation.household.operational?

      invitation.household.household_memberships.active.find_by(account: @account) || raise(Unavailable)
    end
  end
end
