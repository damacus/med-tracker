# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Home', browser: false do
  fixtures :accounts, :account_otp_keys, :people, :users, :medicines, :dosages, :prescriptions

  before do
    driven_by(:rack_test)
  end

  it 'loads the dashboard as the home page for a signed-in user' do
    # Sign in the user from fixtures
    sign_in(users(:damacus))

    # Visit the root path after signing in
    visit root_path

    # Assert that the dashboard is shown
    aggregate_failures 'dashboard content' do
      expect(page).to have_content('Dashboard')
      # Dashboard quick actions are visible on all viewports
      expect(page).to have_link('Add Medicine', href: new_medicine_path)
      expect(page).to have_link('Add Person', href: new_person_path)
    end
  end
end
