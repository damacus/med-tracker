# frozen_string_literal: true

require 'rails_helper'

RSpec.describe LocationPolicy, type: :policy do
  let(:owner) { household_policy_member(role: :owner) }
  let(:household) { owner.fetch(:household) }
  let(:member) { household_policy_member(role: :member, household: household) }
  let(:location) { household_policy_location(household) }
  let(:other_location) { household_policy_location(household_policy_member(role: :owner).fetch(:household)) }

  it 'allows active household members to view locations in their household' do
    expect(described_class.new(member.fetch(:context), location).index?).to be(true)
    expect(described_class.new(member.fetch(:context), location).show?).to be(true)
    expect(described_class.new(member.fetch(:context), other_location).show?).to be(false)
  end

  it 'allows household managers to manage only their household locations' do
    expect(described_class.new(owner.fetch(:context), Location).create?).to be(true)
    expect(described_class.new(owner.fetch(:context), location).update?).to be(true)
    expect(described_class.new(owner.fetch(:context), location).destroy?).to be(true)
    expect(described_class.new(owner.fetch(:context), other_location).update?).to be(false)
    expect(described_class.new(member.fetch(:context), location).update?).to be(false)
  end

  it 'scopes locations to the active household' do
    location
    other_location

    resolved = described_class::Scope.new(member.fetch(:context), Location.all).resolve

    expect(resolved).to include(location)
    expect(resolved).not_to include(other_location)
  end

  it 'does not authorize legacy users' do
    expect(described_class.new(User.new, location).show?).to be(false)
  end
end
