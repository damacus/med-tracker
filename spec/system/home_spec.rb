# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Home', type: :system do
  fixtures :users

  it 'loads the home page for a signed-in user' do
    # Sign in the user from fixtures
    sign_in(users(:damacus))

    # Visit the root path after signing in
    visit root_path

    # Assert that the page content is correct
    within 'Home' do
      aggregate_failures 'home content' do
        expect(page).to have_content('Medicine Tracker')
        expect(page).to have_link('Medicines', href: medicines_path)
        expect(page).to have_link('People', href: people_path)
      end
    end
  end
end
