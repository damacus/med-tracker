module HouseholdInvitations
  class CreateMembership
    RELATIONSHIPS = { 'parent' => 'parent', 'family_member' => 'family_member',
                      'carer' => 'professional_carer', 'professional' => 'professional_carer' }.freeze

    def initialize(invitation:, account:, person:, request: nil)
      @invitation = invitation
      @account = account
      @person = person
      @request = request
    end

    def call
      membership = access_change(@invitation.invited_by_membership).create_membership!(
        household: @invitation.household, account: @account, person: @person,
        role: @invitation.membership_role, status: :active
      )
      access_change(membership).create_grant!(household: @invitation.household, household_membership: membership,
                                              person: @person, access_level: :manage, relationship_type: :self,
                                              granted_by_membership: membership)
      @invitation.household_invitation_grants.find_each { |grant| apply_grant(membership, grant) }
      membership
    end

    private

    def apply_grant(membership, grant)
      relationship = RELATIONSHIPS[grant.relationship_type]
      return create_manual_grant(membership, grant) unless relationship

      CareDelegation::Assign.new(carer: @person, patient: grant.person, relationship_type: relationship,
                                 access_level: grant.access_level, expires_at: grant.expires_at,
                                 granted_by_membership: @invitation.invited_by_membership).call
    end

    def create_manual_grant(membership, grant)
      access_change(@invitation.invited_by_membership).create_grant!(
        household: @invitation.household, household_membership: membership, person: grant.person,
        access_level: grant.access_level, relationship_type: grant.relationship_type, expires_at: grant.expires_at,
        granted_by_membership: @invitation.invited_by_membership
      )
    end

    def access_change(membership)
      Households::AccessChange.for(membership, request: @request)
    end
  end
end
