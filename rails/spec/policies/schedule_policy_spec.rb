# frozen_string_literal: true

require 'rails_helper'

RSpec.describe SchedulePolicy, type: :policy do
  let(:member) { household_policy_member(role: :member) }
  let(:household) { member.fetch(:household) }
  let(:person) { household_policy_person(household) }
  let(:schedule) { household_policy_schedule(household, person: person) }

  it 'uses grant levels for schedule reads, records, and management' do
    expect(policy_results(%i[show?])).to eq(show?: false)

    grant_policy_person_access(member, person, access_level: :view)

    expect(policy_results(%i[show? take_medication?])).to eq(show?: true, take_medication?: false)

    person.person_access_grants.find_by!(household_membership: member.fetch(:membership)).update!(access_level: :record)

    expect(policy_results(%i[take_medication? update?])).to eq(take_medication?: true, update?: false)

    person.person_access_grants.find_by!(household_membership: member.fetch(:membership)).update!(access_level: :manage)

    expect(policy_results(%i[update? destroy?])).to eq(update?: true, destroy?: true)
  end

  it 'allows managers to create schedules without a target record' do
    owner = household_policy_member(role: :owner, household: household)

    expect(described_class.new(owner.fetch(:context), Schedule).create?).to be(true)
    expect(described_class.new(member.fetch(:context), Schedule).create?).to be(false)
  end

  it 'scopes schedules to granted people' do
    schedule
    other_schedule = household_policy_schedule(household, person: household_policy_person(household))
    grant_policy_person_access(member, person, access_level: :view)

    resolved = described_class::Scope.new(member.fetch(:context), Schedule.all).resolve

    expect(resolved).to contain_exactly(schedule)
    expect(resolved).not_to include(other_schedule)
  end

  def policy_results(actions)
    actions.index_with { |action| described_class.new(member.fetch(:context), schedule).public_send(action) }
  end
end
