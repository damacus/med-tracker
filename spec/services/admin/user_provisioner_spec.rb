# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Admin::UserProvisioner do
  fixtures :accounts, :households, :people, :users, :carer_relationships

  subject(:result) do
    described_class.new(
      user: user,
      password: user.password,
      household: household,
      actor_membership: actor_membership
    ).call
  end

  before { FixtureHouseholdSetup.apply! }

  let(:household) { households(:fixture_household) }
  let(:actor_membership) do
    household.household_memberships.find_by!(account: users(:admin).person.account)
  end
  let(:dependent) { people(:child_patient) }
  let(:user) do
    User.new(
      email_address: 'provisioned.user@example.test',
      password: 'SecureP@ssword123!',
      password_confirmation: 'SecureP@ssword123!',
      membership_role: 'administrator',
      dependent_access_level: 'manage',
      dependent_relationship_type: 'parent',
      dependent_ids: [dependent.id],
      person_attributes: {
        name: 'Provisioned User',
        date_of_birth: '1990-01-01'
      }
    )
  end

  it 'creates the account, membership, self grant, and dependant grant atomically' do
    expect { result }
      .to change(Account, :count).by(1)
      .and change(User, :count).by(1)
      .and change(HouseholdMembership, :count).by(1)
      .and change(PersonAccessGrant, :count).by(2)
      .and change(CarerRelationship, :count).by(1)

    expect(result).to have_attributes(success?: true, error: nil, user: user)
  end

  it 'connects the new account to its household access records' do
    result

    account = Account.find_by!(email: user.email_address)
    membership = household.household_memberships.find_by!(account: account)
    expect(account).to be_verified
    expect(user.reload.person).to have_attributes(account: account, household: household)
    expect(membership).to have_attributes(person: user.person, role: 'administrator', status: 'active')
    expect(grant_for(membership, user.person))
      .to have_attributes(access_level: 'manage', relationship_type: 'self')
    relationship = CarerRelationship.find_by!(carer: user.person, patient: dependent)
    expect(grant_for(membership, dependent))
      .to have_attributes(access_level: 'manage', relationship_type: 'parent', carer_relationship: relationship)
  end

  it 'preserves the selected dependent access level' do
    user.dependent_access_level = 'view'

    expect(result).to be_success

    membership = household.household_memberships.find_by!(account: user.person.account)
    relationship = CarerRelationship.find_by!(carer: user.person, patient: dependent)
    expect(grant_for(membership, dependent))
      .to have_attributes(access_level: 'view', relationship_type: 'parent', carer_relationship: relationship)
  end

  it 'rejects an unsupported dependent access level without writing partial records' do
    user.dependent_access_level = 'unsupported'

    expect { result }.not_to(change { provisioned_record_counts })

    expect(result).to have_attributes(success?: false, error: :invalid_access_level, user: user)
    expect(user.errors[:dependent_access_level]).to include('is not included in the list')
  end

  it 'audits membership creation at the initial permissions version' do
    result

    membership = household.household_memberships.find_by!(account: user.person.account)
    event = SecurityAuditEvent.where(event_type: 'household_access.membership_created').order(:id).last
    expect(event.metadata).to include(
      'target_membership_id' => membership.id,
      'new_state' => include('permissions_version' => 1)
    )
  end

  it 'rejects duplicate accounts without writing partial records' do
    user.email_address = users(:jane).email_address

    expect { result }.not_to(
      change { [Account.count, User.count, HouseholdMembership.count, PersonAccessGrant.count] }
    )

    expect(result).to have_attributes(success?: false, error: :duplicate_account, user: user)
    expect(user.errors[:email_address]).to include('has already been taken')
  end

  it 'rejects owner membership creation without writing partial records' do
    user.membership_role = 'owner'

    expect { result }.not_to(
      change { [Account.count, User.count, HouseholdMembership.count, PersonAccessGrant.count] }
    )

    expect(result).to have_attributes(success?: false, error: :invalid_membership_role, user: user)
    expect(user.errors[:membership_role]).to include(
      I18n.t('admin.membership_roles.owner_rejected')
    )
  end

  def grant_for(membership, person)
    household.person_access_grants.find_by!(household_membership: membership, person: person)
  end

  def provisioned_record_counts
    [Account, User, HouseholdMembership, PersonAccessGrant, CarerRelationship].map(&:count)
  end
end
