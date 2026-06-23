# frozen_string_literal: true

require 'rails_helper'

RSpec.describe LocationMembershipPolicy, type: :policy do
  it 'permits create? for household managers only' do
    owner = household_policy_member(role: :owner)
    member = household_policy_member(role: :member, household: owner.fetch(:household))

    expect(described_class.new(owner.fetch(:context), LocationMembership).create?).to be(true)
    expect(described_class.new(member.fetch(:context), LocationMembership).create?).to be(false)
    expect(described_class.new(User.new, LocationMembership).create?).to be(false)
    expect(described_class.new(nil, LocationMembership).create?).to be(false)
  end

  it 'permits destroy? for household managers only inside their household' do
    owner = household_policy_member(role: :owner)
    other_owner = household_policy_member(role: :owner)
    household = owner.fetch(:household)
    location = household.locations.create!(name: 'Policy Location')
    membership = household.location_memberships.create!(person: owner.fetch(:person), location: location)

    expect(described_class.new(owner.fetch(:context), membership).destroy?).to be(true)
    expect(described_class.new(other_owner.fetch(:context), membership).destroy?).to be(false)
    expect(described_class.new(User.new, membership).destroy?).to be(false)
  end
end
