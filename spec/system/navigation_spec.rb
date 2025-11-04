# frozen_string_literal: true

require 'rails_helper'

# This system test verifies the main site navigation using the Capybara DSL.
RSpec.describe 'Navigation' do
  fixtures :users, :people

  before do
    driven_by(:rack_test)
  end

  let(:user) { users(:one) }

  context 'when user is authenticated' do
    it 'shows navigation with a sign out button' do
      # Use our new Capybara-based helper to sign in.
      sign_in(user)

      # Assert that the navigation bar contains the correct elements.
      within('nav') do
        aggregate_failures 'navigation bar' do
          expect(page).to have_link('Medicines')
          expect(page).to have_link('People')
          expect(page).to have_link('Medicine Finder')
          expect(page).to have_button('Sign out')
          expect(page).to have_no_link('Login')
        end
      end
    end
  end

  context 'when user is not authenticated' do
    it 'shows navigation with a login link' do
      # Navigate to the root path.
      visit root_path

      # Assert that the navigation bar contains the correct elements for a guest.
      within('nav') do
        aggregate_failures 'navigation bar' do
          # Unauthenticated users should not see navigation links
          expect(page).to have_no_link('Medicines')
          expect(page).to have_no_link('People')
          expect(page).to have_no_link('Medicine Finder')
          # But they should see the login link
          expect(page).to have_link('Login')
          expect(page).to have_no_button('Sign out')
        end
      end
    end
  end
end
