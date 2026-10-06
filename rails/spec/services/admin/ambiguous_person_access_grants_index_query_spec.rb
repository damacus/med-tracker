# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Admin::AmbiguousPersonAccessGrantsIndexQuery, type: :service do
  include HouseholdPolicyHelpers

  subject(:results) { described_class.new(scope: PersonAccessGrant.where(household: household)).call }

  let(:household) { household_policy_member(role: :owner).fetch(:household) }
  let(:carer_member) { household_policy_member(role: :member, household: household, name: 'Queue Carer') }
  let(:patient) { household_policy_person(household, name: 'Queue Patient', person_type: :minor, has_capacity: false) }
  let(:grant) do
    CarerRelationship.create!(
      household: household, carer: carer_member.fetch(:person), patient: patient,
      relationship_type: :parent, active: true
    )
    PersonAccessGrant.create!(
      household: household,
      household_membership: carer_member.fetch(:membership),
      person: patient,
      access_level: :manage,
      relationship_type: :professional,
      created_at: 2.minutes.ago
    )
  end

  it 'returns active source-null grants with compatible active relationships' do
    expect(results).to contain_exactly(grant)
    expect(results.first.association(:person)).to be_loaded
    expect(results.first.association(:household_membership)).to be_loaded
    expect(results.first.household_membership.association(:person)).to be_loaded
    expect(results.first[:compatible_relationship_type]).to eq('parent')
  end

  it 'excludes revoked, expired, classified, and incompatible grants' do
    revoked = grant.dup
    revoked.revoked_at = Time.current
    revoked.save!(validate: false)
    create_expired_grant
    create_classified_grant
    create_incompatible_grant

    expect(results).to contain_exactly(grant)
  end

  it 'keeps descriptive grant relationship types independent from compatibility' do
    expect(grant.relationship_type).to eq('professional')
    expect(results.first[:compatible_relationship_type]).to eq('parent')
  end

  it 'orders by newest creation and id without duplicate rows' do
    grant.update!(created_at: Time.current)
    newer_member = household_policy_member(role: :member, household: household, name: 'Newer Carer')
    newer_patient = household_policy_person(household, name: 'Newer Patient', person_type: :minor, has_capacity: false)
    _newer_relationship = CarerRelationship.create!(
      household: household,
      carer: newer_member.fetch(:person),
      patient: newer_patient,
      relationship_type: :parent,
      active: true
    )
    newer = create_grant(newer_member, newer_patient, created_at: 1.minute.from_now)

    expect(results.to_a).to eq([newer, grant])
  end

  it 'excludes a grant at its expiry boundary' do
    freeze_time do
      grant.update!(expires_at: Time.current)

      expect(results).to be_empty
    end
  end

  def create_grant(member, person, **attributes)
    PersonAccessGrant.create!(
      {
        household: household,
        household_membership: member.fetch(:membership),
        person: person,
        access_level: :manage,
        relationship_type: :professional
      }.merge(attributes)
    )
  end

  def create_expired_grant
    member = household_policy_member(role: :member, household: household, name: 'Expired Carer')
    person = household_policy_person(household, name: 'Expired Patient', person_type: :minor, has_capacity: false)
    create_grant(member, person, expires_at: 1.minute.ago)
  end

  def create_classified_grant
    member = household_policy_member(role: :member, household: household, name: 'Classified Carer')
    person = household_policy_person(household, name: 'Classified Patient', person_type: :minor, has_capacity: false)
    relationship = CarerRelationship.create!(household: household, carer: member.fetch(:person), patient: person,
                                             relationship_type: :parent, active: true)
    create_grant(member, person, carer_relationship: relationship)
  end

  def create_incompatible_grant
    member = household_policy_member(role: :member, household: household, name: 'Incompatible Carer')
    person = household_policy_person(household, name: 'Other Patient', person_type: :minor, has_capacity: false)
    create_grant(member, person)
  end
end
