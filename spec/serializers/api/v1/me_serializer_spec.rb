# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Api::V1::MeSerializer do
  fixtures :all

  # users(:john) has person: john and account: john_doe — different fixture names so different IDs
  let(:user) { users(:john) }
  let(:json) { described_class.new(user).as_json }

  before do
    household = Household.create!(name: 'Me Household', slug: 'me-household')
    user.person.update!(household: household)
    Current.membership = household.household_memberships.create!(
      account: user.person.account,
      person: user.person,
      role: :owner,
      status: :active
    )
  end

  after { Current.reset }

  it 'serialises the user and membership role' do
    expect(json).to include(
      id: user.id,
      email_address: user.email_address,
      membership_role: 'owner',
      active: user.active
    )
  end

  it 'serialises the nested person' do
    expect(json[:person]).to eq(Api::V1::PersonSerializer.new(user.person).as_json)
  end

  it 'serialises the nested account' do
    account = user.person.account
    expect(json[:account]).to include(
      id: account.id, email: account.email, status: account.status
    )
    # account.id must differ from person.id to distinguish account_data mutation
    expect(json.dig(:account, :id)).to eq(account.id)
    expect(json.dig(:account, :id)).not_to eq(user.person.id)
  end
end
