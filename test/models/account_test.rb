# frozen_string_literal: true

require "test_helper"

class AccountTest < ActiveSupport::TestCase
  # Disable fixtures for this test class
  setup do
    @disable_transactional_tests = true
  end

  test "creates account with valid attributes" do
    account = Account.new(
      email: "test@example.com",
      password_hash: "hashed_password",
      status: :unverified
    )

    assert account.valid?
    assert account.save
  end

  test "requires unique email" do
    Account.create!(email: "test@example.com", password_hash: "hash", status: :unverified)
    
    duplicate = Account.new(email: "test@example.com", password_hash: "hash2", status: :unverified)
    
    assert_not duplicate.valid?
  end

  test "can have associated person" do
    account = Account.create!(email: "test@example.com", password_hash: "hash", status: :verified)
    person = Person.create!(
      name: "Test User",
      date_of_birth: 25.years.ago,
      email: "test@example.com",
      account: account
    )

    assert_equal person, account.person
    assert_equal account, person.account
  end

  test "can have multiple identities" do
    account = Account.create!(email: "test@example.com", password_hash: "hash", status: :verified)
    
    identity1 = account.account_identities.create!(
      provider: "google",
      uid: "12345",
      info: "{}"
    )
    
    identity2 = account.account_identities.create!(
      provider: "facebook",
      uid: "67890",
      info: "{}"
    )

    assert_equal 2, account.account_identities.count
    assert_includes account.account_identities, identity1
    assert_includes account.account_identities, identity2
  end

  test "delegates name to person" do
    account = Account.create!(email: "test@example.com", password_hash: "hash", status: :verified)
    person = Person.create!(
      name: "Jane Doe",
      date_of_birth: 25.years.ago,
      email: "test@example.com",
      account: account
    )

    assert_equal "Jane Doe", account.name
  end

  test "handles status enum correctly" do
    account = Account.new(email: "test@example.com", password_hash: "hash")
    
    account.status = :unverified
    assert account.unverified?
    
    account.status = :verified
    assert account.verified?
    
    account.status = :closed
    assert account.closed?
  end
end
