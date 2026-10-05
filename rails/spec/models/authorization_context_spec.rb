# frozen_string_literal: true

require 'rails_helper'

RSpec.describe AuthorizationContext do
  let(:account) { Account.create!(email: 'context-account@example.test', status: :verified) }
  let(:household) do
    Household.create_with_owner!(
      name: 'Context Family',
      owner_account: account,
      owner_person_attributes: {
        name: 'Context Owner',
        date_of_birth: 30.years.ago.to_date,
        person_type: :adult,
        has_capacity: true
      }
    )
  end
  let(:membership) { household.household_memberships.sole }

  it 'carries account, household, and membership as the Pundit user object' do
    context = described_class.new(account: account, household: household, membership: membership)

    expect(context.account).to eq(account)
    expect(context.household).to eq(household)
    expect(context.membership).to eq(membership)
  end

  it 'builds a context from Current when tenant state is complete' do
    Current.account = account
    Current.household = household
    Current.membership = membership

    context = described_class.current

    expect(context).to have_attributes(account: account, household: household, membership: membership)
  ensure
    Current.reset
  end

  it 'returns nil when Current tenant state is incomplete' do
    Current.account = Account.new

    expect(described_class.current).to be_nil
  ensure
    Current.reset
  end
end
