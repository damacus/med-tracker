# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Mobile Navigation' do
  let(:user) { create(:user, name: 'Test User') }

  before do
    login_as(user)
  end

  scenario 'User sees mobile-specific navigation elements' do
    page.current_window.resize_to(375, 667)
    visit root_path

    # Mobile menu button (hamburger) should be visible
    expect(page).to have_css('button[aria-label="Open menu"]')

    # Desktop menu links should be hidden
    expect(page).to have_no_link('Medicines', class: 'nav__link')

    # Bottom navigation bar should be visible
    expect(page).to have_css('nav.mobile-nav')
    expect(page).to have_link('Home', href: root_path)
    expect(page).to have_link('Medicines', href: medicines_path)
    expect(page).to have_link('People', href: people_path)
  end

  scenario 'User can open the mobile menu' do
    page.current_window.resize_to(375, 667)
    visit root_path

    # Open sheet
    find('button[aria-label="Open menu"]').click

    # Links in the drawer
    within('[role="dialog"]') do
      expect(page).to have_link('Medicines', href: medicines_path)
      expect(page).to have_link('People', href: people_path)
      expect(page).to have_link('Medicine Finder', href: medicine_finder_path)
      expect(page).to have_link('Profile', href: profile_path)
      expect(page).to have_button('Logout')
    end
  end
end
