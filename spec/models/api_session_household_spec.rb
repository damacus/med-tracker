# frozen_string_literal: true

require 'rails_helper'

RSpec.describe ApiSession do
  def household_bundle
    account = Account.create!(email: "api-session-#{SecureRandom.hex(4)}@example.test", status: :verified)
    household = Household.create_with_owner!(
      name: "API Session #{SecureRandom.hex(4)}",
      owner_account: account,
      owner_person_attributes: {
        name: 'API Owner',
        date_of_birth: 30.years.ago.to_date,
        person_type: :adult,
        has_capacity: true
      }
    )
    [account, household.household_memberships.sole]
  end

  def add_backup_owner(household)
    account = Account.create!(email: "api-session-backup-#{SecureRandom.hex(4)}@example.test", status: :verified)
    household.household_memberships.create!(account: account, role: :owner, status: :active)
  end

  it 'binds issued tokens to a household membership and permissions version' do
    account, membership = household_bundle

    session, = described_class.issue_for(account: account, household_membership: membership)

    expect(session.household_membership).to eq(membership)
    expect(session.permissions_version).to eq(membership.permissions_version)
    expect(session.active_for_membership?).to be(true)
  end

  it 'treats sessions without a household membership as inactive' do
    session = described_class.new(household_membership: nil)

    expect(session.active_for_membership?).to be(false)
  end

  it 'partitions token audit versions by household membership' do
    account, membership = household_bundle

    described_class.issue_for(account: account, household_membership: membership)

    version = PaperTrail::Version.where(item_type: 'AuthenticationToken').last
    expect(version.household_id).to eq(membership.household_id)
    expect(version.actor_membership_id).to eq(membership.id)
  end

  it 'invalidates when membership is revoked or permissions change' do
    account, membership = household_bundle
    session, = described_class.issue_for(account: account, household_membership: membership)

    membership.update!(permissions_version: membership.permissions_version + 1)
    expect(session.reload.active_for_membership?).to be(false)

    session.update!(permissions_version: membership.permissions_version)
    add_backup_owner(membership.household)
    membership.update!(status: 'revoked')
    expect(session.reload.active_for_membership?).to be(false)
  end

  it 'rejects binding a token to another account membership' do
    account, = household_bundle
    _, other_membership = household_bundle

    expect do
      described_class.issue_for(account: account, household_membership: other_membership)
    end.to raise_error(ActiveRecord::RecordInvalid)
  end
end
