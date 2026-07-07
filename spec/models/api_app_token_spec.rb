# frozen_string_literal: true

require 'rails_helper'

RSpec.describe ApiAppToken do
  fixtures :accounts

  describe '#touch_last_used!' do
    let(:account) { accounts(:jane_doe) }
    let(:membership) { api_membership_for(account) }
    let(:issued_at) { Time.zone.parse('2026-04-21 10:00:00') }

    it 'skips the write when the token was used recently' do
      app_token = issue_app_token_at_issued_time

      travel_to(issued_at + 1.minute) do
        expect do
          app_token.touch_last_used!
        end.not_to(change { app_token.reload.updated_at })
      end
    end

    it 'updates the timestamp when the token has not been used recently' do
      app_token = issue_app_token_at_issued_time

      travel_to(issued_at + 10.minutes) do
        expect do
          app_token.touch_last_used!
        end.to change { app_token.reload.last_used_at }.from(issued_at).to(Time.current)
      end
    end
  end

  describe '#active_for_membership?' do
    let(:account) { accounts(:jane_doe) }
    let(:membership) { api_membership_for(account) }

    it 'requires an attached active membership with the current permissions version' do
      app_token = described_class.issue_for(
        account: account,
        household_membership: membership,
        name: 'RSpec token'
      ).first

      expect(app_token).to be_active_for_membership

      app_token.update!(permissions_version: membership.permissions_version + 1)
      expect(app_token).not_to be_active_for_membership

      app_token.update!(permissions_version: membership.permissions_version)
      create_backup_owner
      membership.update!(status: :revoked)
      expect(app_token).not_to be_active_for_membership
    end

    it 'rejects tokens with no membership' do
      app_token = described_class.new(household_membership: nil)

      expect(app_token).not_to be_active_for_membership
    end
  end

  describe 'validations' do
    let(:account) { accounts(:jane_doe) }
    let(:household) { api_household }

    it 'rejects household memberships from a different account' do
      other_account = Account.create!(email: 'api-token-other-account@example.test', status: :verified)
      token = described_class.new(
        account: account,
        household_membership: mismatched_membership(other_account),
        name: 'Mismatched token',
        token_digest: described_class.digest('mismatched-token'),
        last_used_at: Time.current
      )

      expect(token).not_to be_valid
      expect(token.errors[:household_membership]).to include('must belong to the account')
    end
  end

  def issue_app_token_at_issued_time
    travel_to(issued_at) do
      described_class.issue_for(
        account: account,
        household_membership: membership,
        name: 'RSpec token'
      ).first
    end
  end

  def api_membership_for(account)
    household = api_household
    household.household_memberships.create!(
      account: account,
      person: api_person_for(account, household),
      role: :owner,
      status: :active
    )
  end

  def api_household
    Household.create!(name: "API App Token Spec #{SecureRandom.hex(4)}",
                      slug: "api-app-token-spec-#{SecureRandom.hex(4)}")
  end

  def api_person_for(account, household)
    Person.create!(
      household: household,
      account: account,
      name: 'API App Token Person',
      date_of_birth: 30.years.ago.to_date,
      person_type: :adult,
      has_capacity: true
    )
  end

  def mismatched_membership(account)
    household.household_memberships.create!(
      account: account,
      person: api_person_for(account, household),
      role: :member,
      status: :active
    )
  end

  def create_backup_owner
    backup_account = Account.create!(email: "api-backup-owner-#{SecureRandom.hex(4)}@example.test", status: :verified)
    membership.household.household_memberships.create!(
      account: backup_account,
      person: api_person_for(backup_account, membership.household),
      role: :owner,
      status: :active
    )
  end
end
