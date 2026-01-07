# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Mobile Navigation' do
  fixtures :accounts, :people, :users

  let(:user) { users(:bob) }

  before do
    login_as(user)
  end

  scenario 'User sees mobile navigation elements on small viewport' do
    page.current_window.resize_to(375, 667)
    visit root_path

    expect(page).to have_css('button[aria-label="Open menu"]')
    expect(page).to have_css('nav.mobile-nav')
    expect(page).to have_link('Home', href: root_path)
    expect(page).to have_link('Medicines', href: medicines_path)
    expect(page).to have_link('People', href: people_path)
  end

  scenario 'Desktop navigation is hidden on mobile viewport' do
    page.current_window.resize_to(375, 667)
    visit root_path

    # Wait for page to stabilize before checking absence
    expect(page).to have_css('nav.mobile-nav')
    expect(page).to have_no_link('Medicines', class: 'nav__link')
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
