# frozen_string_literal: true

require 'rails_helper'
require 'pundit/rspec'

RSpec.describe MedicationTakePolicy, type: :policy do
  let(:member) { household_policy_member(role: :member) }
  let(:household) { member.fetch(:household) }
  let(:person) { household_policy_person(household) }
  let(:schedule) { household_policy_schedule(household, person: person) }
  let(:take) { MedicationTake.new(schedule: schedule, dose_amount: 500, dose_unit: 'mg', taken_at: Time.current) }

  it 'requires record access to create takes' do
    expect(described_class.new(member.fetch(:context), take)).to forbid_action(:create)

    grant_policy_person_access(member, person, access_level: :view)

    expect(described_class.new(member.fetch(:context), take)).to forbid_action(:create)

    person.person_access_grants.find_by!(household_membership: member.fetch(:membership)).update!(access_level: :record)

    expect(described_class.new(member.fetch(:context), take)).to permit_action(:create)
    expect(described_class.new(member.fetch(:context), take)).to permit_action(:new)
  end

  it 'scopes takes to manager households or granted people' do
    owner = household_policy_member(role: :owner, household: household)
    persisted_take = household_policy_medication_take(household, source: schedule)
    other_person = household_policy_person(household)
    other_take = household_policy_medication_take(
      household,
      source: household_policy_schedule(household, person: other_person)
    )
    grant_policy_person_access(member, person, access_level: :view)

    manager_scope = described_class::Scope.new(owner.fetch(:context), MedicationTake.all).resolve
    member_scope = described_class::Scope.new(member.fetch(:context), MedicationTake.all).resolve

    expect(manager_scope).to include(persisted_take, other_take)
    expect(member_scope).to contain_exactly(persisted_take)
  end
end
