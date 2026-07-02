# frozen_string_literal: true

require 'rails_helper'
require 'pundit/rspec'

RSpec.describe HealthEventPolicy, type: :policy do
  let(:member) { household_policy_member(role: :member) }
  let(:household) { member.fetch(:household) }
  let(:person) { household_policy_person(household) }
  let(:event) { HealthEvent.new(person: person, event_kind: :illness, title: 'Flu', started_on: Time.zone.today) }

  it 'requires record access to create health events' do
    expect(described_class.new(member.fetch(:context), event)).to forbid_action(:create)

    grant_policy_person_access(member, person, access_level: :view)

    expect(described_class.new(member.fetch(:context), event)).to forbid_action(:create)

    person.person_access_grants.find_by!(household_membership: member.fetch(:membership)).update!(access_level: :record)

    expect(described_class.new(member.fetch(:context), event)).to permit_action(:create)
    expect(described_class.new(member.fetch(:context), event)).to permit_action(:new)
  end

  it 'requires manage access to update or destroy health events' do
    grant_policy_person_access(member, person, access_level: :record)

    expect(described_class.new(member.fetch(:context), event)).to forbid_action(:update)
    expect(described_class.new(member.fetch(:context), event)).to forbid_action(:destroy)

    person.person_access_grants.find_by!(household_membership: member.fetch(:membership)).update!(access_level: :manage)

    expect(described_class.new(member.fetch(:context), event)).to permit_action(:update)
    expect(described_class.new(member.fetch(:context), event)).to permit_action(:destroy)
  end

  it 'scopes events to manager households or granted people' do
    owner = household_policy_member(role: :owner, household: household)
    event = HealthEvent.create!(person: person, event_kind: :illness, title: 'Flu', started_on: Time.zone.today)
    other_event = HealthEvent.create!(
      person: household_policy_person(household),
      event_kind: :suspected_side_effect,
      title: 'Rash',
      started_on: Time.zone.today
    )
    grant_policy_person_access(member, person, access_level: :view)

    manager_scope = described_class::Scope.new(owner.fetch(:context), HealthEvent.all).resolve
    member_scope = described_class::Scope.new(member.fetch(:context), HealthEvent.all).resolve

    expect(manager_scope).to include(event, other_event)
    expect(member_scope).to contain_exactly(event)
  end
end
