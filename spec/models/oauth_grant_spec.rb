# frozen_string_literal: true

require 'rails_helper'

RSpec.describe OauthGrant do
  it 'rejects pre-hosted grants without a membership or permissions version' do
    membership = HouseholdMembership.new(status: :active, permissions_version: 1)

    expect(described_class.new(household_membership: nil, permissions_version: 1)).not_to be_active_for_membership
    expect(described_class.new(household_membership: membership, permissions_version: nil))
      .not_to be_active_for_membership
  end
end
