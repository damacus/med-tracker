# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Households::AccessChange do
  fixtures :accounts

  let(:household) { create_household('Access Change') }
  let(:owner_membership) { create_membership(household, accounts(:admin), :owner) }
  let(:member_membership) { create_membership(household, accounts(:jane_doe), :member) }
  let(:service) do
    described_class.new(
      actor_account: owner_membership.account,
      actor_membership: owner_membership,
      request: nil
    )
  end

  describe '#update_membership' do
    it 'advances permissions exactly once for a role change' do
      expect_membership_change(role: :administrator)
    end

    it 'advances permissions exactly once for a status change' do
      expect_membership_change(status: :suspended)
    end

    it 'advances permissions exactly once for a person change' do
      expect_membership_change(person: create_person(household, nil, 'Replacement Person'))
    end

    it 'does not advance permissions for a no-op' do
      original_version = member_membership.permissions_version

      result = service.update_membership(member_membership, role: member_membership.role)

      expect(result).to have_attributes(success?: true, outcome: 'no_change')
      expect(member_membership.reload.permissions_version).to eq(original_version)
    end

    it 'rolls back the membership and version when audit persistence fails' do
      original_version = member_membership.permissions_version
      allow(Audit::Event).to receive(:record!).and_raise(ActiveRecord::RecordInvalid.new(SecurityAuditEvent.new))

      expect do
        service.update_membership(member_membership, role: :administrator)
      end.to raise_error(ActiveRecord::RecordInvalid)

      expect(member_membership.reload).to have_attributes(role: 'member', permissions_version: original_version)
    end

    it 'rejects promotion to owner without an active platform administrator' do
      expect do
        result = service.update_membership(member_membership, role: :owner)
        expect(result).not_to be_success
      end.not_to(change { member_membership.reload.permissions_version })

      expect(member_membership).not_to be_owner
      expect(member_membership.errors[:base]).to include('Owner promotion requires an active platform administrator')
    end

    it 'allows an active platform administrator to promote an owner' do
      platform_account = accounts(:damacus)
      PlatformAdmin.create!(account: platform_account)
      platform_service = described_class.new(actor_account: platform_account, actor_membership: nil, request: nil)

      expect do
        expect(platform_service.update_membership(member_membership, role: :owner)).to be_success
      end.to change { member_membership.reload.permissions_version }.by(1)

      expect(member_membership).to be_owner
    end

    it 'rejects removing the last active owner' do
      expect do
        result = service.update_membership(owner_membership, role: :member)
        expect(result).not_to be_success
      end.not_to(change { owner_membership.reload.permissions_version })

      expect(owner_membership.errors[:base]).to include('Last active owner cannot be removed')
    end

    it 'rejects an actor membership from another household' do
      foreign_household = create_household('Foreign Actor')
      foreign_owner = create_membership(foreign_household, accounts(:damacus), :owner)
      foreign_service = described_class.new(
        actor_account: foreign_owner.account,
        actor_membership: foreign_owner,
        request: nil
      )

      result = foreign_service.update_membership(member_membership, role: :administrator)

      expect(result).not_to be_success
      expect(member_membership.reload).to be_member
      expect(member_membership.errors[:base]).to include('Access change actor must belong to the household')
    end
  end

  describe 'person access grant changes' do
    let(:person) { create_person(household, nil, 'Dependent Person') }

    it 'advances permissions when creating a grant' do
      expect do
        result = service.create_grant(
          household: household,
          household_membership: member_membership,
          person: person,
          access_level: :view,
          relationship_type: :family_member,
          granted_by_membership: owner_membership
        )
        expect(result).to be_success
      end.to change { member_membership.reload.permissions_version }.by(1)
    end

    it 'advances permissions when changing a grant' do
      grant = create_grant(member_membership, person, :view)

      expect do
        expect(service.update_grant(grant, access_level: :manage)).to be_success
      end.to change { member_membership.reload.permissions_version }.by(1)

      expect(grant.reload).to be_manage
    end

    it 'advances permissions when revoking a grant' do
      grant = create_grant(member_membership, person, :view)

      expect do
        expect(service.revoke_grant(grant)).to be_success
      end.to change { member_membership.reload.permissions_version }.by(1)

      expect(grant.reload.revoked_at).to be_present
    end

    it 'does not advance permissions when a grant change is a no-op' do
      grant = create_grant(member_membership, person, :view)
      original_version = member_membership.permissions_version

      result = service.update_grant(grant, access_level: :view)

      expect(result).to have_attributes(success?: true, outcome: 'no_change')
      expect(member_membership.reload.permissions_version).to eq(original_version)
    end

    it 'rolls back the grant and version when audit persistence fails' do
      grant = create_grant(member_membership, person, :view)
      original_version = member_membership.permissions_version
      allow(Audit::Event).to receive(:record!).and_raise(ActiveRecord::RecordInvalid.new(SecurityAuditEvent.new))

      expect do
        service.update_grant(grant, access_level: :manage)
      end.to raise_error(ActiveRecord::RecordInvalid)

      expect(grant.reload).to be_view
      expect(member_membership.reload.permissions_version).to eq(original_version)
    end

    it 'rejects a grant that crosses household boundaries and audits the rejection' do
      foreign_household = create_household('Foreign Grant')
      foreign_person = create_person(foreign_household, nil, 'Foreign Person')

      expect do
        result = service.create_grant(grant_attributes(foreign_person))
        expect(result).not_to be_success
      end.to change {
        SecurityAuditEvent.where(event_type: 'household_access.person_grant_changed').count
      }.by(1)

      event = SecurityAuditEvent.where(event_type: 'household_access.person_grant_changed').order(:id).last
      expect(event.metadata).to include('outcome' => 'rejected', 'target_membership_id' => member_membership.id)
      expect(event.metadata.keys).not_to include('person_name', 'medication_name', 'notes')
    end
  end

  def create_household(label)
    Household.create!(name: "#{label} #{SecureRandom.hex(4)}", slug: "#{label.parameterize}-#{SecureRandom.hex(4)}")
  end

  def expect_membership_change(attributes)
    expect do
      expect(service.update_membership(member_membership, attributes)).to be_success
    end.to change { member_membership.reload.permissions_version }.by(1)
  end

  def grant_attributes(person)
    {
      household: household,
      household_membership: member_membership,
      person: person,
      access_level: :view,
      relationship_type: :family_member,
      granted_by_membership: owner_membership
    }
  end

  def create_membership(target_household, account, role)
    person = create_person(target_household, account, "#{role.to_s.titleize} Person")
    target_household.household_memberships.create!(account: account, person: person, role: role, status: :active)
  end

  def create_person(target_household, account, name)
    Person.create!(
      household: target_household,
      account: account,
      name: name,
      date_of_birth: 30.years.ago.to_date,
      person_type: :adult,
      has_capacity: true
    )
  end

  def create_grant(membership, person, access_level)
    PersonAccessGrant.create!(
      household: membership.household,
      household_membership: membership,
      person: person,
      access_level: access_level,
      relationship_type: :family_member,
      granted_by_membership: owner_membership
    )
  end
end
