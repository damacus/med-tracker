# frozen_string_literal: true

require 'rails_helper'

RSpec.describe PlatformAdmin do
  it 'belongs to an account and is active by default' do
    account = Account.create!(email: 'operator@example.test', status: :verified)
    admin = described_class.create!(account: account)

    expect(admin).to be_active
    expect(account.platform_admin).to eq(admin)
  end

  it 'requires one platform admin row per account' do
    account = Account.create!(email: 'unique-operator@example.test', status: :verified)
    described_class.create!(account: account)

    duplicate = described_class.new(account: account)

    expect(duplicate).not_to be_valid
    expect(duplicate.errors[:account_id]).to include('has already been taken')
  end
end
