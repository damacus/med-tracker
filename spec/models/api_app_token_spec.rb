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
end
