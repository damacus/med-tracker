# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Home' do
  fixtures :users, :people, :medicines, :dosages, :prescriptions

  it 'loads the dashboard as the home page for a signed-in user' do
    # Sign in the user from fixtures
    sign_in(users(:damacus))

    # Visit the root path after signing in
    visit root_path

    # Assert that the dashboard is shown
    aggregate_failures 'dashboard content' do
      expect(page).to have_content('Dashboard')
      expect(page).to have_link('Medicines', href: medicines_path)
      expect(page).to have_link('People', href: people_path)
      expect(page).to have_link('Add Medicine', href: new_medicine_path)
    end
  end
end
