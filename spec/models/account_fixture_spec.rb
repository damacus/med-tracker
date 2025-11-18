# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Account do
  fixtures :accounts

  it 'creates Account records with correct password hashes' do
    # Check if Account fixtures are loaded
    expect(described_class.count).to be > 0

    # Get the first account
    account = described_class.first

    # Verify the account has required fields
    expect(account.email).to be_present
    expect(account.password_hash).to be_present
    expect(account.status).to eq('verified') # Rodauth stores status as string

    # Test password verification works using BCrypt directly
    expect(BCrypt::Password.new(account.password_hash) == 'password').to be true
  end
end
