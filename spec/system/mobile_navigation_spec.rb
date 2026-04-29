# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Mobile Navigation' do
  fixtures :accounts, :people, :users

  let(:user) { users(:bob) }

  before do
    login_as(user)
  end

  scenario 'renders the mobile navigation shell on a small viewport' do
    page.current_window.resize_to(375, 667)
    visit root_path

    expect(page).to have_css('button[aria-label="Open menu"]')
    expect(page).to have_css('nav.mobile-nav')
    expect(page).to have_link('Home', href: root_path)
    expect(page).to have_link('Inventory', href: medications_path)
    expect(page).to have_link('Reports', href: reports_path)
    expect(page).to have_link('Profile', href: profile_path)
    expect(page).to have_no_css('aside')
  end

  scenario 'opens a left-side drawer with navigation and accessible sizing' do
    page.current_window.resize_to(375, 667)
    visit root_path

    find('button[aria-label="Open menu"]').click

    within('[role="dialog"]') do
      expect(page).to have_link('Inventory', href: medications_path)
      expect(page).to have_link('People', href: people_path)
      expect(page).to have_link('Medication Finder', href: medication_finder_path)
      expect(page).to have_link('Profile', href: profile_path)
      expect(page).to have_button('Logout')
    end

    drawer = find('[role="dialog"]')
    drawer_left = drawer.evaluate_script('this.getBoundingClientRect().left')
    expect(drawer_left).to eq(0)

    expect(page).to have_css('[data-testid="drawer-backdrop"]')

    drawer_width = drawer.evaluate_script('this.getBoundingClientRect().width')
    viewport_width = 375.0

    ratio = drawer_width / viewport_width
    expect(ratio).to be_between(0.70, 0.85)

    expect(drawer[:'aria-modal']).to eq('true')
    expect(drawer[:'aria-label']).to be_present

    within('[role="dialog"]') do
      all('a, button').each do |target|
        height = target.evaluate_script('this.getBoundingClientRect().height')
        expect(height).to be >= 24, "Touch target '#{target.text}' height #{height}px < 24px minimum"
      end
    end
  end

  scenario 'dismisses the drawer with backdrop and Escape and can reopen' do
    page.current_window.resize_to(375, 667)
    visit root_path

    find('button[aria-label="Open menu"]').click
    expect(page).to have_css('[role="dialog"]')

    page.execute_script("document.querySelector('[data-testid=\"drawer-backdrop\"]').click()")
    expect(page).to have_no_css('[role="dialog"]')

    find('button[aria-label="Open menu"]').click
    expect(page).to have_css('[role="dialog"]')

    find('body').send_keys(:escape)
    expect(page).to have_no_css('[role="dialog"]')

    find('button[aria-label="Open menu"]').click
    expect(page).to have_css('[role="dialog"]')
  end
end
