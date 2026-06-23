# frozen_string_literal: true

require 'rails_helper'

RSpec.describe HouseholdInvitation do
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

  def build_invitation(household:, membership:, email: 'invitee@example.test', **attributes)
    described_class.new(
      {
        household: household,
        invited_by_membership: membership,
        email: email,
        membership_role: :member
      }.merge(attributes)
    )
  end

  it 'generates a token and expiry before creation' do
    household, membership = household_bundle(email: 'household-inviter@example.test', name: 'Invitation Token')
    invitation = build_invitation(household: household, membership: membership)

    invitation.save!

    expect(invitation.token).to be_present
    expect(invitation.token_digest).to eq(described_class.digest(invitation.token))
    expect(invitation.expires_at).to be > Time.current
  end

  it 'tracks pending and expired states' do
    household, membership = household_bundle(email: 'states-inviter@example.test', name: 'Invitation States')
    pending = build_invitation(household: household, membership: membership, email: 'pending@example.test')
    expired = build_invitation(
      household: household,
      membership: membership,
      email: 'expired@example.test',
      expires_at: 1.day.ago
    )

    [pending, expired].each(&:save!)

    expect(described_class.pending).to contain_exactly(pending)
    expect(described_class.expired).to contain_exactly(expired)
  end

  it 'tracks accepted and revoked terminal states' do
    household, membership = household_bundle(email: 'terminal-inviter@example.test', name: 'Invitation Terminal')
    accepted = build_invitation(household: household, membership: membership, email: 'accepted@example.test',
                                accepted_at: Time.current)
    revoked = build_invitation(household: household, membership: membership, email: 'revoked@example.test',
                               revoked_at: Time.current)

    [accepted, revoked].each(&:save!)

    expect(accepted).to be_accepted
    expect(revoked).to be_revoked
  end

  it 'rotates the token and records a resend version' do
    household, membership = household_bundle(email: 'resend-inviter@example.test', name: 'Invitation Resend')
    invitation = build_invitation(household: household, membership: membership)
    invitation.save!
    original_digest = invitation.token_digest

    expect do
      invitation.resend!
    end.to change(PaperTrail::Version.where(item_type: 'HouseholdInvitation'), :count).by(1)

    expect(invitation.token_digest).not_to eq(original_digest)
    expect(invitation.token).to be_present
    expect(invitation.versions.last.event).to eq('resend')
  end

  it 'prevents duplicate active invitations within one household' do
    household, membership = household_bundle(email: 'duplicate-inviter@example.test', name: 'Invitation Duplicate')
    build_invitation(household: household, membership: membership, email: 'duplicate@example.test').save!
    duplicate = build_invitation(household: household, membership: membership, email: 'duplicate@example.test')

    expect(duplicate).not_to be_valid
    expect(duplicate.errors[:email]).to include('has already been taken')
  end
end
