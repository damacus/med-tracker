# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationPolicy, type: :policy do
  let(:owner) { household_policy_member(role: :owner) }
  let(:household) { owner.fetch(:household) }
  let(:member) { household_policy_member(role: :member, household: household) }
  let(:person) { household_policy_person(household) }
  let(:medication) { household_policy_medication(household) }
  let(:other_medication) { household_policy_medication(household_policy_member(role: :owner).fetch(:household)) }

  it 'allows active household members to reach the medication index' do
    expect(described_class.new(member.fetch(:context), Medication).index?).to be(true)
  end

  it 'shows medication only through household management or explicit person grants' do
    expect(described_class.new(owner.fetch(:context), medication).show?).to be(true)
    expect(described_class.new(member.fetch(:context), medication).show?).to be(false)

    household_policy_schedule(household, person: person, medication: medication)
    grant_policy_person_access(member, person, access_level: :view)

    expect(described_class.new(member.fetch(:context), medication).show?).to be(true)
    expect(described_class.new(member.fetch(:context), other_medication).show?).to be(false)
  end

  it 'allows managers to update and destroy household medications' do
    expect(described_class.new(owner.fetch(:context), medication).update?).to be(true)
    expect(described_class.new(owner.fetch(:context), medication).destroy?).to be(true)
    expect(described_class.new(owner.fetch(:context), other_medication).update?).to be(false)
    expect(described_class.new(member.fetch(:context), medication).update?).to be(false)
  end

  it 'allows only household managers to run a stock check' do
    administrator = household_policy_member(role: :administrator, household: household)

    expect(described_class.new(owner.fetch(:context), Medication).stock_check?).to be(true)
    expect(described_class.new(administrator.fetch(:context), Medication).stock_check?).to be(true)
    expect(described_class.new(member.fetch(:context), Medication).stock_check?).to be(false)
  end

  it 'allows creation for managers or members with manage grants' do
    location = household_policy_location(household)

    expect(described_class.new(owner.fetch(:context), Medication.new(location: location)).create?).to be(true)
    expect(described_class.new(member.fetch(:context), Medication.new(location: location)).create?).to be(false)

    grant_policy_person_access(member, person, access_level: :manage)

    expect(described_class.new(member.fetch(:context), Medication.new(location: location)).create?).to be(true)
  end

  it 'scopes medications to household managers or granted people' do
    medication
    other_medication
    granted_medication = household_policy_medication(household)
    household_policy_person_medication(household, person: person, medication: granted_medication)
    grant_policy_person_access(member, person, access_level: :view)

    manager_scope = described_class::Scope.new(owner.fetch(:context), Medication.all).resolve
    member_scope = described_class::Scope.new(member.fetch(:context), Medication.all).resolve

    expect(manager_scope).to include(medication, granted_medication)
    expect(manager_scope).not_to include(other_medication)
    expect(member_scope).to contain_exactly(granted_medication)
  end

  it 'does not expand member medication scope beyond granted people for manage grants' do
    medication
    granted_medication = household_policy_medication(household)
    household_policy_person_medication(household, person: person, medication: granted_medication)
    grant_policy_person_access(member, person, access_level: :manage)

    member_scope = described_class::Scope.new(member.fetch(:context), Medication.all).resolve

    expect(member_scope).to contain_exactly(granted_medication)
  end
end
