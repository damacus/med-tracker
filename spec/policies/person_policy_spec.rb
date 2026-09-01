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

    expect(policy_results(%i[show? update? download_health_history?], person)).to eq(
      show?: true,
      update?: false,
      download_health_history?: false
    )

    person.person_access_grants.find_by!(household_membership: member.fetch(:membership)).update!(access_level: :manage)

    expect(policy_results(%i[update? destroy? add_medication? download_health_history?], person)).to eq(
      update?: true,
      destroy?: true,
      add_medication?: true,
      download_health_history?: true
    )
    expect(policy_results(%i[show?], other_person)).to eq(show?: false)
  end

  describe '#download_health_history?' do
    it 'allows manage grants for administrator, clinician, self, carer, and parent access' do
      access_contexts = [
        [household_policy_member(role: :administrator, household: household), :family_member],
        [household_policy_member(role: :member, household: household), :professional],
        [member, :self],
        [household_policy_member(role: :member, household: household), :carer],
        [household_policy_member(role: :member, household: household), :parent]
      ]

      results = access_contexts.map do |access_member, relationship_type|
        grant_policy_person_access(access_member, person, access_level: :manage, relationship_type:)
        described_class.new(access_member.fetch(:context), person).download_health_history?
      end

      expect(results).to all(be(true))
    end

    it 'denies record, view, missing, and cross-household access' do
      record_member = household_policy_member(role: :member, household: household)
      view_member = household_policy_member(role: :member, household: household)
      ungranted_member = household_policy_member(role: :member, household: household)
      grant_policy_person_access(record_member, person, access_level: :record, relationship_type: :professional)
      grant_policy_person_access(view_member, person, access_level: :view, relationship_type: :carer)

      results = [record_member, view_member, ungranted_member].map do |access_member|
        described_class.new(access_member.fetch(:context), person).download_health_history?
      end
      results << described_class.new(member.fetch(:context), other_person).download_health_history?

      expect(results).to all(be(false))
    end
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

  it 'scopes people by the requested access level' do
    view_person = household_policy_person(household, name: 'View Person')
    record_person = household_policy_person(household, name: 'Record Person')
    manage_person = household_policy_person(household, name: 'Manage Person')
    grant_policy_person_access(member, view_person, access_level: :view)
    grant_policy_person_access(member, record_person, access_level: :record)
    grant_policy_person_access(member, manage_person, access_level: :manage)

    scope = described_class::Scope.new(member.fetch(:context), Person.all)

    expect(scope.resolve_for(:manage)).to contain_exactly(manage_person)
    expect(scope.resolve_for(:record)).to contain_exactly(record_person, manage_person)
    expect(scope.resolve).to contain_exactly(view_person, record_person, manage_person)
  end

  it 'does not authorize legacy users' do
    expect(described_class.new(User.new, person).index?).to be(false)
  end

  def policy_results(actions, record)
    actions.index_with { |action| described_class.new(member.fetch(:context), record).public_send(action) }
  end
end
