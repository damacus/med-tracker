# frozen_string_literal: true

module Households
  class LocalMembershipMigrator
    def initialize(household:, owner:, accounts:, prepare_person:, role_for:)
      @household = household
      @owner = owner
      @accounts = accounts
      @prepare_person = prepare_person
      @role_for = role_for
    end

    def call
      owner_membership = upsert_membership(owner, nil)
      accounts.find_each do |account|
        upsert_membership(account, owner_membership) unless account == owner
      end
    end

    private

    attr_reader :household, :owner, :accounts, :prepare_person, :role_for

    def upsert_membership(account, owner_membership)
      person = account.person
      prepare_person.call(person)
      attributes = membership_attributes(account, person)
      membership = HouseholdMembership.find_by(household: household, account: account)
      return update_membership(membership, owner_membership, attributes) if membership

      access_change(account, owner_membership).create_membership!(household: household, **attributes)
    end

    def membership_attributes(account, person)
      {
        account: account,
        person: person,
        role: role_for.call(account),
        status: :active,
        joined_at: Time.current
      }
    end

    def update_membership(membership, owner_membership, attributes)
      actor_membership = owner_membership || membership
      Households::AccessChange.for(actor_membership).update_membership!(membership, attributes.except(:account))
    end

    def access_change(account, owner_membership)
      return Households::AccessChange.for(owner_membership) if owner_membership

      Households::AccessChange.new(actor_account: account, actor_membership: nil, request: nil)
    end
  end
end
