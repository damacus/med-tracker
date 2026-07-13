# frozen_string_literal: true

require 'rails_helper'

RSpec.describe CareDelegation::Assign do
  subject(:assign_delegation) { assigner.call }

  def assigner
    described_class.new(
      carer: carer,
      patient: patient,
      relationship_type: 'parent',
      granted_by_membership: actor_membership
    )
  end

  let(:household) { create(:household) }
  let(:carer) { create_account_person(household, 'carer') }
  let(:patient) { create(:person, household: household) }
  let(:actor) { create_account_person(household, 'actor') }
  let!(:actor_membership) { create_membership(actor, role: :owner) }

  it 'atomically creates the relationship, membership, self grant, and patient grant' do
    expect { assign_delegation }
      .to change(CarerRelationship, :count).by(1)
      .and change(HouseholdMembership, :count).by(1)
      .and change(PersonAccessGrant, :count).by(2)

    relationship = CarerRelationship.find_by!(carer: carer, patient: patient)
    membership = household.household_memberships.find_by!(account: carer.account)
    self_grant = membership.person_access_grants.find_by!(person: carer)
    patient_grant = membership.person_access_grants.find_by!(person: patient)

    expect_created_assignment(relationship, membership, self_grant, patient_grant)
  end

  it 'maps professional carers to record access with professional metadata' do
    relationship = described_class.new(
      carer: carer,
      patient: patient,
      relationship_type: 'professional_carer',
      granted_by_membership: actor_membership
    ).call

    grant = relationship.person_access_grants.sole
    expect(grant).to have_attributes(access_level: 'record', relationship_type: 'professional')
  end

  it 'maps family relationships to manage access with family metadata' do
    relationship = described_class.new(
      carer: carer,
      patient: patient,
      relationship_type: 'family_member',
      granted_by_membership: actor_membership
    ).call

    grant = relationship.person_access_grants.sole
    expect(grant).to have_attributes(access_level: 'manage', relationship_type: 'family_member')
  end

  it 'uses an explicit access level without changing relationship metadata' do
    relationship = described_class.new(
      carer: carer,
      patient: patient,
      relationship_type: 'family_member',
      access_level: :view,
      granted_by_membership: actor_membership
    ).call

    expect(relationship.person_access_grants.sole)
      .to have_attributes(access_level: 'view', relationship_type: 'family_member')
  end

  it 'rejects an unsupported explicit access level without partial writes' do
    original_counts = [CarerRelationship.count, HouseholdMembership.count, PersonAccessGrant.count]

    expect do
      described_class.new(
        carer: carer,
        patient: patient,
        relationship_type: 'parent',
        access_level: :administer,
        granted_by_membership: actor_membership
      ).call
    end.to raise_error(CareDelegation::Assign::InvalidAccessLevel)

    expect([CarerRelationship.count, HouseholdMembership.count, PersonAccessGrant.count]).to eq(original_counts)
  end

  it 'applies the delegated access expiry' do
    expires_at = 2.days.from_now

    relationship = described_class.new(
      carer: carer,
      patient: patient,
      relationship_type: 'parent',
      granted_by_membership: actor_membership,
      expires_at: expires_at
    ).call

    expect(relationship.person_access_grants.sole.expires_at).to be_within(1.second).of(expires_at)
  end

  it 'persists a new dependent through the same transactional assignment' do
    new_patient = build(:person, :minor, household: household)

    relationship = described_class.new(
      carer: carer,
      patient: new_patient,
      relationship_type: 'family_member',
      granted_by_membership: actor_membership
    ).call

    expect(new_patient).to be_persisted
    expect(relationship).to be_persisted
    expect(relationship.person_access_grants.sole.person).to eq(new_patient)
  end

  it 'keeps a self relationship descriptive and the identity grant membership-owned' do
    membership = create_membership(carer)
    identity_grant = create_self_grant(membership)
    original_attributes = identity_grant.attributes

    relationship = described_class.new(
      carer: carer,
      patient: carer,
      relationship_type: :self,
      granted_by_membership: actor_membership
    ).call

    expect(relationship).to be_active
    expect(relationship.person_access_grants).to be_empty
    expect(identity_grant.reload.attributes).to eq(original_attributes)
  end

  it 'releases a legacy sourced self grant back to membership ownership' do
    membership = create_membership(carer)
    relationship = CarerRelationship.create!(household: household, carer: carer, patient: carer,
                                             relationship_type: :self, active: true)
    identity_grant = create_self_grant(membership)
    identity_grant.update!(carer_relationship: relationship)

    result = described_class.new(carer: carer, patient: carer, relationship_type: :self).call

    expect(result).to eq(relationship)
    expect(identity_grant.reload).to have_attributes(carer_relationship_id: nil, revoked_at: nil)
  end

  it 'creates only the descriptive relationship for an accountless carer' do
    accountless_carer = create(:person, household: household)
    original_counts = [HouseholdMembership.count, PersonAccessGrant.count]

    relationship = described_class.new(
      carer: accountless_carer,
      patient: patient,
      relationship_type: :family_member,
      granted_by_membership: actor_membership
    ).call

    expect(relationship).to have_attributes(carer: accountless_carer, patient: patient, active: true)
    expect([HouseholdMembership.count, PersonAccessGrant.count]).to eq(original_counts)
  end

  it 'reactivates the same relationship and owned grant' do
    relationship = assign_delegation
    grant = relationship.person_access_grants.sole
    CareDelegation::Revoke.new(relationship: relationship).call

    expect { assigner.call }
      .not_to(change { [CarerRelationship.count, PersonAccessGrant.count] })

    expect(relationship.reload).to be_active
    expect(grant.reload.revoked_at).to be_nil
  end

  it 'is idempotent for an active assignment' do
    relationship = assigner.call
    grant = relationship.person_access_grants.sole

    expect { assigner.call }
      .not_to(change { [CarerRelationship.count, HouseholdMembership.count, PersonAccessGrant.count] })

    expect(assigner.call).to eq(relationship)
    expect(relationship.reload.person_access_grants.sole).to eq(grant)
  end

  it 'rejects cross-household assignments without partial writes' do
    foreign_carer = create_account_person(create(:household), 'foreign-carer')

    original_counts = [CarerRelationship.count, HouseholdMembership.count, PersonAccessGrant.count]

    expect do
      described_class.new(
        carer: foreign_carer,
        patient: patient,
        relationship_type: 'parent',
        granted_by_membership: actor_membership
      ).call
    end.to raise_error(ActiveRecord::RecordInvalid)

    expect([CarerRelationship.count, HouseholdMembership.count, PersonAccessGrant.count]).to eq(original_counts)
  end

  it 'rolls back every write when grant persistence fails' do
    foreign_actor = create_account_person(create(:household), 'foreign-actor')
    foreign_membership = create_membership(foreign_actor, role: :owner)

    original_counts = [CarerRelationship.count, HouseholdMembership.count, PersonAccessGrant.count]

    expect do
      described_class.new(
        carer: carer,
        patient: patient,
        relationship_type: 'parent',
        granted_by_membership: foreign_membership
      ).call
    end.to raise_error(ActiveRecord::RecordInvalid)

    expect([CarerRelationship.count, HouseholdMembership.count, PersonAccessGrant.count]).to eq(original_counts)
  end

  it 'preserves a sufficient manually granted access record' do
    membership = create_membership(carer)
    create_self_grant(membership)
    manual_grant = create_manual_grant(membership, access_level: :manage)
    original_attributes = manual_grant.attributes

    relationship = assign_delegation

    expect(manual_grant.reload.attributes).to eq(original_attributes)
    expect(relationship.person_access_grants).to be_empty

    expect { CareDelegation::Revoke.new(relationship: relationship).call }
      .to raise_error(CareDelegation::Revoke::AmbiguousGrant)
    expect(relationship.reload).to be_active
    expect(manual_grant.reload.revoked_at).to be_nil
  end

  it 'rejects an insufficient manual grant without changing either record' do
    membership = create_membership(carer)
    create_self_grant(membership)
    manual_grant = create_manual_grant(membership, access_level: :view)

    original_count = CarerRelationship.count

    expect { assign_delegation }.to raise_error(CareDelegation::Assign::GrantConflict)

    expect(CarerRelationship.count).to eq(original_count)

    expect(manual_grant.reload).to have_attributes(access_level: 'view', revoked_at: nil)
  end

  it 'retires an expired manual grant before creating an owned grant' do
    membership = create_membership(carer)
    create_self_grant(membership)
    manual_grant = create_manual_grant(membership, access_level: :manage, expires_at: 1.minute.ago)

    relationship = assign_delegation

    expect(manual_grant.reload.revoked_at).to be_present
    expect(relationship.person_access_grants.sole)
      .to have_attributes(access_level: 'manage', relationship_type: 'parent', revoked_at: nil)
  end

  def create_account_person(target_household, prefix)
    account = Account.create!(
      email: "#{prefix}-#{SecureRandom.hex(4)}@example.com",
      password_hash: BCrypt::Password.create('password'),
      status: :verified
    )
    create(:person, household: target_household, account: account)
  end

  def expect_created_assignment(relationship, membership, self_grant, patient_grant)
    expect(relationship).to have_attributes(household: household, relationship_type: 'parent', active: true)
    expect(membership).to have_attributes(person: carer, role: 'member', status: 'active')
    expect(self_grant).to have_attributes(access_level: 'manage', relationship_type: 'self', revoked_at: nil)
    expect(patient_grant).to have_attributes(access_level: 'manage', relationship_type: 'parent',
                                             carer_relationship: relationship, revoked_at: nil)
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

  def create_manual_grant(membership, access_level:, expires_at: nil)
    household.person_access_grants.create!(
      household_membership: membership,
      person: patient,
      access_level: access_level,
      relationship_type: :family_member,
      granted_by_membership: actor_membership,
      expires_at: expires_at
    )
  end
end
