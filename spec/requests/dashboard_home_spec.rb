# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Dashboard home rendering' do
  fixtures :accounts, :people, :users

  describe 'GET /' do
    it 'redirects to the current household dashboard' do
      household, = household_membership_for(users(:jane))
      sign_in(users(:jane))

      get root_path

      expect(response).to redirect_to("/households/#{household.slug}/dashboard")
    end

    it 'redirects after login when RLS requires account context for membership lookup' do
      household, = household_membership_for(users(:jane))
      account = users(:jane).person.account
      clear_2fa_for_account(account)

      ActiveRecord::Base.connection.execute('SET LOCAL ROLE med_tracker_app')

      post '/login', params: { email: account.email, password: 'password' }

      expect(response).to redirect_to("/households/#{household.slug}/dashboard")
    end
  end

  describe 'GET /households/:household_slug/dashboard' do
    it 'uses the existing dashboard when no experiment is selected' do
      household, = household_membership_for(users(:jane))
      account = users(:jane).person.account
      account.update!(preferences: account.preferences.except('dashboard_variant'))
      sign_in(users(:jane))

      get "/households/#{household.slug}/dashboard"

      expect(response).to have_http_status(:ok)
      expect(response.parsed_body.at_css('[data-testid="dashboard"]')).to be_present
      expect(response.parsed_body.at_css('[data-testid^="dashboard-variant-"]')).not_to be_present
    end

    {
      'time_first' => 'dashboard-variant-time-first',
      'family_lanes' => 'dashboard-variant-family-lanes',
      'calm_focus' => 'dashboard-variant-calm-focus'
    }.each do |variant, testid|
      it "renders the #{variant.tr('_', ' ')} dashboard experiment" do
        household, = household_membership_for(users(:jane))
        users(:jane).person.account.update!(dashboard_variant: variant)
        sign_in(users(:jane))

        get "/households/#{household.slug}/dashboard"

        expect(response).to have_http_status(:ok)
        expect(response.parsed_body.at_css("[data-testid=\"#{testid}\"]")).to be_present
      end
    end

    it 'renders granted household records and excludes colliding records from another household' do
      household, membership = household_membership_for(users(:jane))
      other_household = Household.create!(name: 'Other Dashboard Household', slug: 'other-dashboard-household')
      home = household.locations.find_or_create_by!(name: 'Home')
      other_home = other_household.locations.create!(name: 'Home')
      jane = grant_person(household, membership, users(:jane).person, access_level: :manage)
      child = grant_person(
        household,
        membership,
        household.people.create!(
          name: 'Alex Dashboard',
          date_of_birth: 30.years.ago.to_date,
          person_type: :adult,
          has_capacity: true
        )
      )
      other_child = other_household.people.create!(
        name: 'Foreign Dashboard Alex',
        date_of_birth: 30.years.ago.to_date,
        person_type: :adult,
        has_capacity: true
      )
      visible_medication = household.medications.create!(name: 'Paracetamol Dashboard', location: home)
      other_medication = other_household.medications.create!(name: 'Foreign Dashboard Medicine', location: other_home)
      create_schedule(household, jane, visible_medication)
      create_schedule(household, child, visible_medication)
      create_schedule(other_household, other_child, other_medication)
      sign_in(users(:jane))

      get "/households/#{household.slug}/dashboard", params: { dashboard_person_id: DashboardPresenter::ALL_FAMILY_PERSON_ID }

      expect(response).to have_http_status(:ok)
      expect(response.body).to include('Alex Dashboard')
      expect(response.body).to include('Paracetamol Dashboard')
      expect(response.body).not_to include('Foreign Dashboard Alex')
      expect(response.body).not_to include('Foreign Dashboard Medicine')
      expect(response.body).not_to include(other_household.slug)
    end
  end

  def household_membership_for(user)
    household = user.person.household
    membership = household.household_memberships.find_or_create_by!(account: user.person.account, person: user.person)
    membership.update!(role: :member, status: :active)

    [household, membership]
  end

  def grant_person(household, membership, person, access_level: :view)
    grant = household.person_access_grants.find_or_initialize_by(
      household_membership: membership,
      person: person,
      revoked_at: nil
    )
    grant.update!(
      access_level: access_level,
      relationship_type: :family_member,
      granted_by_membership: membership
    )
    person
  end

  def create_schedule(household, person, medication)
    household.schedules.create!(
      person: person,
      medication: medication,
      dose_amount: 500,
      dose_unit: 'mg',
      frequency: 'Daily',
      start_date: Time.zone.today,
      end_date: 1.week.from_now.to_date
    )
  end
end
