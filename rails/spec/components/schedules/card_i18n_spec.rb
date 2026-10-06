# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Schedules::Card, type: :component do
  before do
    @account = Account.create!(email: 'schedule-card-i18n-owner@example.test', status: :verified)
    @household = Household.create_with_owner!(
      name: 'Schedule Card I18n Household',
      owner_account: account,
      owner_person_attributes: {
        name: 'Schedule Card Owner',
        date_of_birth: 30.years.ago.to_date,
        person_type: :adult,
        has_capacity: true
      }
    )
    @membership = household.household_memberships.find_by!(account: account)
    @person = membership.person
    @location = household.locations.create!(name: 'Schedule Card I18n Cabinet')
    @medication = household.medications.create!(name: 'Ibuprofen', location: location, reorder_threshold: 0)
    @schedule = Schedule.create!(
      household: household,
      person: person,
      medication: medication,
      dose_amount: 400.0,
      dose_unit: 'mg',
      frequency: 'Twice daily',
      start_date: 1.month.ago,
      end_date: 1.month.from_now
    )
  end

  after do
    Current.reset
  end

  attr_reader :account, :household, :location, :medication, :membership, :person, :schedule

  describe 'i18n translations' do
    it 'renders card with default locale translations' do
      vc = view_context
      vc.singleton_class.define_method(:current_user) { nil }

      html = vc.render(described_class.new(schedule: schedule, person: person))
      rendered = Nokogiri::HTML::DocumentFragment.parse(html)
      text = rendered.text

      expect(text).not_to include("Today's Doses")
      expect(text).not_to include('No doses taken today')
      expect(text).to include('Started')
      expect(text).to include('Ends')
    end

    it 'renders delete dialog with translated strings for admin user' do
      Current.account = account
      Current.household = household
      Current.membership = membership
      vc = view_context
      vc.singleton_class.define_method(:current_user) { nil }

      html = vc.render(described_class.new(schedule: schedule, person: person))
      rendered = Nokogiri::HTML::DocumentFragment.parse(html)
      text = rendered.text

      expect(text).to include('Delete Schedule')
      expect(text).to include('Cancel')
    end
  end
end
