# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Admin user mutation boundary' do
  fixtures :accounts, :people, :users, :locations, :carer_relationships

  let(:admin) { users(:admin) }
  let(:household) { Household.find_by!(slug: default_url_options.fetch(:household_slug)) }

  before do
    sign_in(admin)
    attach_fixture_users_to_household
  end

  it 'keeps generic user update from changing roles or dependent grants' do
    parent = users(:parent)
    dependent = people(:child_patient)
    membership = household.household_memberships.find_by!(account: parent.person.account)
    CarerRelationship.where(carer: parent.person, patient: dependent).destroy_all
    household.person_access_grants.where(household_membership: membership, person: dependent).destroy_all

    patch admin_user_path(parent),
          params: {
            user: {
              email_address: parent.email_address,
              membership_role: 'administrator',
              dependent_access_level: 'manage',
              dependent_relationship_type: 'parent',
              dependent_ids: [dependent.id],
              person_attributes: {
                id: parent.person.id,
                name: 'Parent Profile Update',
                date_of_birth: parent.person.date_of_birth.to_s,
                location_ids: [locations(:home).id]
              }
            }
          }

    expect(response).to redirect_to(admin_users_path)
    expect(parent.person.reload.name).to eq('Parent Profile Update')
    expect(membership.reload.role).to eq('member')
    expect(CarerRelationship.exists?(carer: parent.person, patient: dependent)).to be(false)
    expect(household.person_access_grants.exists?(household_membership: membership, person: dependent)).to be(false)
  end

  it 'updates a membership role through the dedicated endpoint and records an audit event' do
    member = users(:jane)
    membership = household.household_memberships.find_by!(account: member.person.account)

    expect do
      patch membership_role_admin_user_path(member), params: { membership: { role: 'administrator' } }
    end.to change(SecurityAuditEvent, :count).by(1)

    audit_event = SecurityAuditEvent.order(:created_at).last

    expect(response).to redirect_to(admin_users_path)
    expect(membership.reload.role).to eq('administrator')
    expect(audit_event).to have_attributes(
      household: household,
      actor_account: admin.person.account,
      event_type: 'household_membership.role_updated'
    )
    expect(audit_event.metadata).to include(
      'target_account_id' => member.person.account.id,
      'previous_role' => 'member',
      'new_role' => 'administrator'
    )
  end

  it 'rolls back the membership role when audit recording fails' do
    member = users(:jane)
    membership = household.household_memberships.find_by!(account: member.person.account)
    allow(SecurityAuditEvent).to receive(:create!).and_raise(ActiveRecord::RecordInvalid.new(SecurityAuditEvent.new))

    expect do
      patch membership_role_admin_user_path(member), params: { membership: { role: 'administrator' } }
    end.not_to(change { membership.reload.role })

    expect(response).to have_http_status(:unprocessable_content)
  end

  it 'rejects owner promotion through the dedicated household endpoint' do
    member = users(:jane)
    membership = household.household_memberships.find_by!(account: member.person.account)

    patch membership_role_admin_user_path(member), params: { membership: { role: 'owner' } }

    expect(response).to redirect_to(admin_users_path)
    expect(flash[:alert]).to include('Owner promotion requires platform admin support')
    expect(membership.reload.role).to eq('member')
  end

  def attach_fixture_users_to_household
    attach_admin_to_household

    User.where.not(id: admin.id).includes(person: :account).find_each do |user|
      attach_member_to_household(user)
    end
  end

  def attach_admin_to_household
    attach_person_to_household(admin.person)
    upsert_household_membership(admin.person, :owner)
  end

  def attach_member_to_household(user)
    return unless user.person&.account

    attach_person_to_household(user.person)
    upsert_household_membership(user.person, :member)
  end

  def attach_person_to_household(person)
    person.household = household
    person.save!(validate: false)
  end

  def upsert_household_membership(person, role)
    household.household_memberships.find_or_initialize_by(account: person.account).tap do |membership|
      membership.person = person
      membership.role = role
      membership.status = :active
      membership.save!
    end
  end
end
