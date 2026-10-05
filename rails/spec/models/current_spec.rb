# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Current do
  let(:account) { Account.create!(email: 'current-account@example.test', status: :verified) }
  let(:household) do
    Household.create_with_owner!(
      name: 'Current Family',
      owner_account: account,
      owner_person_attributes: {
        name: 'Current Owner',
        date_of_birth: 30.years.ago.to_date,
        person_type: :adult,
        has_capacity: true
      }
    )
  end
  let(:membership) { household.household_memberships.sole }

  after { described_class.reset }

  it 'tracks account, household, membership, and request id for one execution context' do
    described_class.account = account
    described_class.household = household
    described_class.membership = membership
    described_class.request_id = 'req-current'

    expect(described_class.account).to eq(account)
    expect(described_class.household).to eq(household)
    expect(described_class.membership).to eq(membership)
    expect(described_class.request_id).to eq('req-current')
  end
end
