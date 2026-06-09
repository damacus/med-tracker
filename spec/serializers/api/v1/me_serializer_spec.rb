# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Api::V1::MeSerializer do
  fixtures :all

  # users(:john) has person: john and account: john_doe — different fixture names so different IDs
  let(:user) { users(:john) }

  it 'serialises the user, nested person and account' do
    json = described_class.new(user).as_json
    expect(json).to include(id: user.id, email_address: user.email_address, role: user.role, active: user.active)
    expect(json[:person]).to eq(Api::V1::PersonSerializer.new(user.person).as_json)
    account = user.person.account
    expect(json[:account]).to include(
      id: account.id, email: account.email, status: account.status
    )
    # account.id must differ from person.id to distinguish account_data mutation
    expect(json.dig(:account, :id)).to eq(account.id)
    expect(json.dig(:account, :id)).not_to eq(user.person.id)
  end
end
