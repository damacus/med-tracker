# frozen_string_literal: true

require 'rails_helper'

RSpec.describe CareDelegation::Revoke do
  subject(:revoke_delegation) { described_class.new(relationship: relationship).call }

  let(:household) { create(:household) }
  let(:carer) { create_account_person(household, 'carer') }
  let(:patient) { create(:person, household: household) }
  let(:actor_membership) { create_membership(create_account_person(household, 'actor'), role: :owner) }
  let(:relationship) do
    CareDelegation::Assign.new(
      carer: carer,
      patient: patient,
      relationship_type: 'parent',
      granted_by_membership: actor_membership
    ).call
  end

  it 'deactivates the relationship and revokes its owned grant together' do
    patient_grant = relationship.person_access_grants.sole
    membership = relationship.household.household_memberships.find_by!(person: carer)
    self_grant = membership.person_access_grants.find_by!(person: carer)

    expect(revoke_delegation).to eq(relationship)
    expect(relationship.reload).not_to be_active
    expect(patient_grant.reload.revoked_at).to be_present
    expect(self_grant.reload.revoked_at).to be_nil
  end

  it 'deactivates a self relationship without revoking or sourcing its identity grant' do
    membership = create_membership(carer)
    identity_grant = create_self_grant(membership)
    self_relationship = CareDelegation::Assign.new(
      carer: carer,
      patient: carer,
      relationship_type: :self,
      granted_by_membership: actor_membership
    ).call

    expect(described_class.new(relationship: self_relationship).call).to eq(self_relationship)
    expect(self_relationship.reload).not_to be_active
    expect(identity_grant.reload).to have_attributes(carer_relationship_id: nil, revoked_at: nil)
  end

  it 'deactivates an accountless descriptive relationship without access records' do
    accountless_carer = create(:person, household: household)
    descriptive_relationship = CarerRelationship.create!(
      household: household,
      carer: accountless_carer,
      patient: patient,
      relationship_type: :family_member,
      active: true
    )

    expect(described_class.new(relationship: descriptive_relationship).call).to eq(descriptive_relationship)
    expect(descriptive_relationship.reload).not_to be_active
    expect(descriptive_relationship.person_access_grants).to be_empty
  end

  it 'refuses to deactivate while an unowned grant still authorizes the patient' do
    manual_relationship, manual_grant = create_manual_delegation

    expect { described_class.new(relationship: manual_relationship).call }
      .to raise_error(CareDelegation::Revoke::AmbiguousGrant, /unowned grant/)

    expect(manual_relationship.reload).to be_active
    expect(manual_grant.reload.revoked_at).to be_nil
  end

  it 'revokes expired owned grants and releases the unrevoked uniqueness slot' do
    expiring_relationship = create_expiring_delegation
    patient_grant = expiring_relationship.person_access_grants.sole
    patient_grant.update!(expires_at: 1.hour.ago)

    described_class.new(relationship: expiring_relationship).call

    expect(patient_grant.reload.revoked_at).to be_present
    expect { create_replacement_grant(patient_grant.household_membership) }
      .to change(PersonAccessGrant, :count).by(1)
  end

  it 'is idempotent when the assignment is already revoked' do
    patient_grant = relationship.person_access_grants.sole
    revoke_delegation
    relationship_updated_at = relationship.reload.updated_at
    grant_updated_at = patient_grant.reload.updated_at

    expect { described_class.new(relationship: relationship).call }
      .not_to(change { [relationship.reload.updated_at, patient_grant.reload.updated_at] })

    expect(relationship.updated_at).to eq(relationship_updated_at)
    expect(patient_grant.updated_at).to eq(grant_updated_at)
  end

  it 'rolls back relationship deactivation when grant revocation fails' do
    patient_grant = relationship.person_access_grants.sole
    PersonAccessGrant.connection.execute(
      "UPDATE person_access_grants SET access_level = 'invalid' WHERE id = #{patient_grant.id}"
    )

    expect { revoke_delegation }.to raise_error(ActiveRecord::RecordInvalid)

    expect(relationship.reload).to be_active
    expect(patient_grant.reload.revoked_at).to be_nil
  end

  def create_account_person(target_household, prefix)
    account = Account.create!(
      email: "#{prefix}-#{SecureRandom.hex(4)}@example.com",
      password_hash: BCrypt::Password.create('password'),
      status: :verified
    )
    create(:person, household: target_household, account: account)
  end

  def create_membership(person, role: :member)
    person.household.household_memberships.create!(
      account: person.account,
      person: person,
      role: role,
      status: :active
    )
  end

  def create_self_grant(membership)
    membership.household.person_access_grants.create!(
      household_membership: membership,
      person: membership.person,
      access_level: :manage,
      relationship_type: :self,
      granted_by_membership: membership
    )
  end

  def create_manual_delegation
    membership = create_membership(carer)
    grant = household.person_access_grants.create!(
      household_membership: membership,
      person: patient,
      access_level: :manage,
      relationship_type: :parent,
      granted_by_membership: actor_membership
    )
    relationship = CarerRelationship.create!(household: household, carer: carer, patient: patient,
                                             relationship_type: :parent, active: true)
    [relationship, grant]
  end

  def create_expiring_delegation
    CareDelegation::Assign.new(
      carer: carer,
      patient: patient,
      relationship_type: 'parent',
      granted_by_membership: actor_membership,
      expires_at: 1.hour.from_now
    ).call
  end

  def create_replacement_grant(membership)
    household.person_access_grants.create!(
      household_membership: membership,
      person: patient,
      access_level: :view,
      relationship_type: :family_member,
      granted_by_membership: actor_membership
    )
  end
end
