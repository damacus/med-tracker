# frozen_string_literal: true

require 'rails_helper'

# This system test verifies the main site navigation using the Capybara DSL.
RSpec.describe 'Navigation' do
  fixtures :accounts, :people, :users

  before do
    driven_by(:playwright)
  end

  let(:user) { users(:jane) }
  let(:admin) { users(:admin) }

  context 'when user is authenticated' do
    it 'shows the sidebar with navigation links' do
      sign_in(user)

      within('aside') do
        aggregate_failures 'sidebar navigation' do
          expect(page).to have_link('Dashboard')
          expect(page).to have_link('Inventory')
          expect(page).to have_link('People')
          expect(page).to have_link('Finder')
          expect(page).to have_link('Reports')
          expect(page).to have_content(user.name)
          expect(page).to have_button('Sign Out')
        end
      end
    end

    it 'shows Administration link for admin users' do
      sign_in(admin)

      within('aside') do
        expect(page).to have_link('Dashboard')
        expect(page).to have_link('Administration', href: admin_root_path)
      end
    end
  end

  context 'when user is not authenticated' do
    it 'shows navigation with a login link' do
      page.current_window.resize_to(375, 667)
      visit root_path

      expect(page).to have_link('Login')
    end
  end
end
