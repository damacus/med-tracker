# frozen_string_literal: true

require 'rails_helper'

# This system test verifies the main site navigation using the Capybara DSL.
RSpec.describe 'Navigation' do
  fixtures :accounts, :people, :users

  before do
    driven_by(:rack_test)
  end

  let(:user) { users(:jane) }
  let(:admin) { users(:admin) }

  context 'when user is authenticated' do
    it 'shows navigation with a profile dropdown menu' do
      sign_in(user)

      within('nav.nav') do
        aggregate_failures 'navigation bar' do
          expect(page).to have_link('Medicines')
          expect(page).to have_link('People')
          expect(page).to have_link('Medicine Finder')
          expect(page).to have_button(user.name) # Profile dropdown trigger
          expect(page).to have_no_link('Login')
        end
      end
    end

    it 'shows profile dropdown menu with correct items' do
      sign_in(user)

      # Click the profile dropdown trigger
      click_button(user.name)

      # Check dropdown menu items
      aggregate_failures 'dropdown menu items' do
        expect(page).to have_link('Dashboard')
        expect(page).to have_link('Profile')
        expect(page).to have_link('Logout')
        # Regular user should not see Administration link
        expect(page).to have_no_link('Administration')
      end
    end

    it 'shows Administration link for admin users' do
      sign_in(admin)

      click_button(admin.name)

      expect(page).to have_link('Administration')
    end
  end

  context 'when user is not authenticated' do
    it 'shows navigation with a login link' do
      # Navigate to the root path.
      visit root_path

      # Assert that the navigation bar contains the correct elements for a guest.
      within('nav.nav') do
        aggregate_failures 'navigation bar' do
          # Unauthenticated users should not see navigation links
          expect(page).to have_no_link('Medicines')
          expect(page).to have_no_link('People')
          expect(page).to have_no_link('Medicine Finder')
          # But they should see the login link
          expect(page).to have_link('Login')
        end
      end
    end
  end
end
