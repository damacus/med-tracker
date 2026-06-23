# frozen_string_literal: true

require 'rails_helper'

RSpec.describe PersonMedicationPolicy, type: :policy do
  let(:member) { household_policy_member(role: :member) }
  let(:household) { member.fetch(:household) }
  let(:person) { household_policy_person(household) }
  let(:person_medication) { household_policy_person_medication(household, person: person) }

  it 'uses grant levels for person medication reads, records, and management' do
    expect(policy_results(%i[show?])).to eq(show?: false)

    grant_policy_person_access(member, person, access_level: :view)

    expect(policy_results(%i[show? take_medication?])).to eq(show?: true, take_medication?: false)

    person.person_access_grants.find_by!(household_membership: member.fetch(:membership)).update!(access_level: :record)

    expect(policy_results(%i[take_medication? update?])).to eq(take_medication?: true, update?: false)

    person.person_access_grants.find_by!(household_membership: member.fetch(:membership)).update!(access_level: :manage)

    expect(policy_results(%i[create? update? destroy?])).to eq(create?: true, update?: true, destroy?: true)
  end

  it 'allows managers to open new person medication forms without a target record' do
    owner = household_policy_member(role: :owner, household: household)

    expect(described_class.new(owner.fetch(:context), PersonMedication).new?).to be(true)
    expect(described_class.new(member.fetch(:context), PersonMedication).new?).to be(false)
  end

  it 'scopes person medications to granted people' do
    person_medication
    other = household_policy_person_medication(household, person: household_policy_person(household))
    grant_policy_person_access(member, person, access_level: :view)

    resolved = described_class::Scope.new(member.fetch(:context), PersonMedication.all).resolve

    expect(resolved).to contain_exactly(person_medication)
    expect(resolved).not_to include(other)
  end

  def policy_results(actions)
    actions.index_with { |action| described_class.new(member.fetch(:context), person_medication).public_send(action) }
  end
end
