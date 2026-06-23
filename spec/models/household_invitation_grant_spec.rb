# frozen_string_literal: true

require 'rails_helper'

RSpec.describe HouseholdInvitationGrant do
  def household_bundle(email:, name:)
    account = Account.create!(email: email, status: :verified)
    household = Household.create_with_owner!(
      name: name,
      owner_account: account,
      owner_person_attributes: {
        name: "#{name} Owner",
        date_of_birth: 30.years.ago.to_date,
        person_type: :adult,
        has_capacity: true
      }
    )
    [household, household.household_memberships.sole]
  end

  def invitation_for(household, membership, email:)
    HouseholdInvitation.create!(
      household: household,
      invited_by_membership: membership,
      email: email,
      token_digest: SecureRandom.hex(32),
      expires_at: 2.days.from_now,
      membership_role: :member
    )
  end

  def person_for(household, name:)
    household.people.create!(
      name: name,
      date_of_birth: 30.years.ago.to_date,
      person_type: :adult,
      has_capacity: true
    )
  end

  it 'allows invitation grants when all linked records belong to one household' do
    household, membership = household_bundle(email: 'invitation-grant-owner@example.test',
                                             name: 'Invitation Grant Household')
    invitation = invitation_for(household, membership, email: 'invitee@example.test')
    person = person_for(household, name: 'Invitee Dependent')

    grant = described_class.new(
      household: household,
      household_invitation: invitation,
      person: person,
      access_level: :record,
      relationship_type: :carer
    )

    expect(grant).to be_valid
  end

  it 'rejects invitation grants linked to another household invitation' do
    household, = household_bundle(email: 'grant-primary-owner@example.test', name: 'Primary Grant Household')
    other_household, other_membership = household_bundle(email: 'grant-other-owner@example.test',
                                                         name: 'Other Grant Household')
    other_invitation = invitation_for(other_household, other_membership, email: 'other-invitee@example.test')
    person = person_for(household, name: 'Primary Person')

    grant = described_class.new(
      household: household,
      household_invitation: other_invitation,
      person: person,
      access_level: :view,
      relationship_type: :family_member
    )

    expect(grant).not_to be_valid
    expect(grant.errors[:household_invitation]).to include('must belong to the same household')
  end

  it 'rejects invitation grants linked to another household person' do
    household, membership = household_bundle(email: 'grant-person-owner@example.test', name: 'Grant Person Household')
    other_household, = household_bundle(email: 'grant-person-other-owner@example.test',
                                        name: 'Grant Person Other Household')
    invitation = invitation_for(household, membership, email: 'person-invitee@example.test')
    other_person = other_household.people.sole

    grant = described_class.new(
      household: household,
      household_invitation: invitation,
      person: other_person,
      access_level: :manage,
      relationship_type: :parent
    )

    expect(grant).not_to be_valid
    expect(grant.errors[:person]).to include('must belong to the same household')
  end

  it 'rejects invitation grants without linked invitation and person records' do
    household, = household_bundle(email: 'grant-missing-owner@example.test', name: 'Grant Missing Household')

    grant = described_class.new(
      household: household,
      access_level: :view,
      relationship_type: :family_member
    )

    expect(grant).not_to be_valid
    expect(grant.errors[:household_invitation]).to include('must belong to the same household')
    expect(grant.errors[:person]).to include('must belong to the same household')
  end
end
