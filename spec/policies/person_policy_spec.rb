# frozen_string_literal: true

require 'rails_helper'

RSpec.describe PersonPolicy, type: :policy do
  let(:member) { household_policy_member(role: :member) }
  let(:household) { member.fetch(:household) }
  let(:person) { household_policy_person(household) }
  let(:other_person) { household_policy_person(household_policy_member(role: :owner).fetch(:household)) }

  it 'uses explicit person grants for reads and writes' do
    expect(policy_results(%i[show?], person)).to eq(show?: false)

    grant_policy_person_access(member, person, access_level: :view)

    expect(policy_results(%i[show? update?], person)).to eq(show?: true, update?: false)

    person.person_access_grants.find_by!(household_membership: member.fetch(:membership)).update!(access_level: :manage)

    expect(policy_results(%i[update? destroy? add_medication?], person)).to eq(
      update?: true,
      destroy?: true,
      add_medication?: true
    )
    expect(policy_results(%i[show?], other_person)).to eq(show?: false)
  end

  it 'allows managers or members with manage grants to create people in their household' do
    owner = household_policy_member(role: :owner, household: household)
    new_person = household.people.build(name: 'New Person', date_of_birth: 5.years.ago.to_date, person_type: :minor)

    expect(described_class.new(owner.fetch(:context), new_person).create?).to be(true)
    expect(described_class.new(member.fetch(:context), new_person).create?).to be(false)

    grant_policy_person_access(member, person, access_level: :manage)

    expect(described_class.new(member.fetch(:context), new_person).create?).to be(true)
    expect(described_class.new(member.fetch(:context), other_person).create?).to be(false)
  end

  it 'scopes people to active grants in the household' do
    person
    other_person
    grant_policy_person_access(member, person, access_level: :view)

    resolved = described_class::Scope.new(member.fetch(:context), Person.all).resolve

    expect(resolved).to contain_exactly(person)
  end

  it 'does not authorize legacy users' do
    expect(described_class.new(User.new, person).index?).to be(false)
  end

  def policy_results(actions, record)
    actions.index_with { |action| described_class.new(member.fetch(:context), record).public_send(action) }
  end
end
