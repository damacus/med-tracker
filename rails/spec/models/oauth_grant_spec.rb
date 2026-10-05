# frozen_string_literal: true

require 'rails_helper'

RSpec.describe OauthGrant do
  it 'rejects pre-hosted grants without a membership or permissions version' do
    membership = HouseholdMembership.new(status: :active, permissions_version: 1)

    expect(described_class.new(household_membership: nil, permissions_version: 1)).not_to be_active_for_membership
    expect(described_class.new(household_membership: membership, permissions_version: nil))
      .not_to be_active_for_membership
  end

  it 'rejects grants for every non-operational household lifecycle state' do
    household = Household.new(status: :active, lifecycle_state: :active)
    membership = HouseholdMembership.new(
      household: household,
      status: :active,
      permissions_version: 1
    )
    grant = described_class.new(household_membership: membership, permissions_version: 1)

    expect(grant).to be_active_for_membership

    %i[held offboarded purging purged].each do |lifecycle_state|
      household.lifecycle_state = lifecycle_state

      expect(grant).not_to be_active_for_membership
    end
  end
end
