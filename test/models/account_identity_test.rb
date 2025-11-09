# frozen_string_literal: true

require "test_helper"

class AccountIdentityTest < ActiveSupport::TestCase
  # Disable fixtures for this test class
  setup do
    @disable_transactional_tests = true
  end

  test "creates identity with valid attributes" do
    account = Account.create!(email: "test@example.com", password_hash: "hash", status: :verified)
    identity = AccountIdentity.new(
      account: account,
      provider: "google",
      uid: "12345",
      info: "{}"
    )

    assert identity.valid?
    assert identity.save
  end

  test "requires provider" do
    account = Account.create!(email: "test@example.com", password_hash: "hash", status: :verified)
    identity = AccountIdentity.new(account: account, uid: "12345", info: "{}")

    assert_not identity.valid?
    assert_includes identity.errors[:provider], "can't be blank"
  end

  test "requires uid" do
    account = Account.create!(email: "test@example.com", password_hash: "hash", status: :verified)
    identity = AccountIdentity.new(account: account, provider: "google", info: "{}")

    assert_not identity.valid?
    assert_includes identity.errors[:uid], "can't be blank"
  end

  test "requires unique provider and uid combination" do
    account1 = Account.create!(email: "test1@example.com", password_hash: "hash", status: :verified)
    account2 = Account.create!(email: "test2@example.com", password_hash: "hash", status: :verified)

    AccountIdentity.create!(account: account1, provider: "google", uid: "12345", info: "{}")
    
    duplicate = AccountIdentity.new(account: account2, provider: "google", uid: "12345", info: "{}")
    
    assert_not duplicate.valid?
    assert_includes duplicate.errors[:uid], "has already been taken"
  end

  test "allows same uid with different provider" do
    account = Account.create!(email: "test@example.com", password_hash: "hash", status: :verified)

    identity1 = AccountIdentity.create!(account: account, provider: "google", uid: "12345", info: "{}")
    identity2 = AccountIdentity.new(account: account, provider: "facebook", uid: "12345", info: "{}")

    assert identity2.valid?
  end

  test "belongs to account" do
    account = Account.create!(email: "test@example.com", password_hash: "hash", status: :verified)
    identity = AccountIdentity.create!(
      account: account,
      provider: "google",
      uid: "12345",
      info: "{}"
    )

    assert_equal account, identity.account
  end
end
