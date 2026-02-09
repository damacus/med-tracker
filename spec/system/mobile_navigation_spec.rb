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

  scenario 'Slide-out drawer is positioned on the left side' do
    page.current_window.resize_to(375, 667)
    visit root_path

    find('button[aria-label="Open menu"]').click

    drawer = find('[role="dialog"]')
    drawer_left = drawer.evaluate_script('this.getBoundingClientRect().left')
    expect(drawer_left).to eq(0)
  end

  scenario 'Slide-out drawer has a semi-transparent backdrop' do
    page.current_window.resize_to(375, 667)
    visit root_path

    find('button[aria-label="Open menu"]').click

    expect(page).to have_css('[data-testid="drawer-backdrop"]')
  end

  scenario 'Tapping backdrop dismisses the drawer' do
    page.current_window.resize_to(375, 667)
    visit root_path

    find('button[aria-label="Open menu"]').click
    expect(page).to have_css('[role="dialog"]')

    page.execute_script("document.querySelector('[data-testid=\"drawer-backdrop\"]').click()")
    expect(page).to have_no_css('[role="dialog"]')
  end

  scenario 'Pressing Escape dismisses the drawer' do
    page.current_window.resize_to(375, 667)
    visit root_path

    find('button[aria-label="Open menu"]').click
    expect(page).to have_css('[role="dialog"]')

    find('body').send_keys(:escape)
    expect(page).to have_no_css('[role="dialog"]')
  end

  scenario 'Drawer width is approximately 75-80% of viewport' do
    page.current_window.resize_to(375, 667)
    visit root_path

    find('button[aria-label="Open menu"]').click

    drawer = find('[role="dialog"]')
    drawer_width = drawer.evaluate_script('this.getBoundingClientRect().width')
    viewport_width = 375.0

    ratio = drawer_width / viewport_width
    expect(ratio).to be_between(0.70, 0.85)
  end

  scenario 'Drawer has accessible aria attributes' do
    page.current_window.resize_to(375, 667)
    visit root_path

    find('button[aria-label="Open menu"]').click

    drawer = find('[role="dialog"]')
    expect(drawer[:'aria-modal']).to eq('true')
    expect(drawer[:'aria-label']).to be_present
  end

  scenario 'Drawer can be reopened after dismissal' do
    page.current_window.resize_to(375, 667)
    visit root_path

    find('button[aria-label="Open menu"]').click
    expect(page).to have_css('[role="dialog"]')

    find('body').send_keys(:escape)
    expect(page).to have_no_css('[role="dialog"]')

    find('button[aria-label="Open menu"]').click
    expect(page).to have_css('[role="dialog"]')
  end

  scenario 'Touch targets in drawer meet WCAG minimum size' do
    page.current_window.resize_to(375, 667)
    visit root_path

    find('button[aria-label="Open menu"]').click

    within('[role="dialog"]') do
      all('a, button').each do |target|
        height = target.evaluate_script('this.getBoundingClientRect().height')
        expect(height).to be >= 24, "Touch target '#{target.text}' height #{height}px < 24px minimum"
      end
    end
  end
end
