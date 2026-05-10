# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Mobile Navigation' do
  fixtures :accounts, :people, :users

  let(:user) { users(:bob) }
  let(:navigation_visibility_script) do
    <<~JS
      (() => {
        const visible = (element) => {
          if (!element) return false

          const styles = window.getComputedStyle(element)
          const bounds = element.getBoundingClientRect()

          return styles.display !== 'none' &&
            styles.visibility !== 'hidden' &&
            bounds.width > 0 &&
            bounds.height > 0
        }

        const bottomNav = document.querySelector('[data-testid="mobile-bottom-nav"]')
        const sidebar = document.querySelector('aside')

        return {
          bottom_nav: visible(bottomNav),
          sidebar: visible(sidebar),
          header: visible(document.querySelector('header')),
          fab: visible(document.querySelector('[data-testid="floating-action-menu-toggle"]'))
        }
      })()
    JS
  end

  before do
    login_as(user)
  end

  scenario 'renders the mobile navigation shell on a small viewport' do
    page.current_window.resize_to(375, 667)
    visit root_path

    expect(page).to have_css('button[aria-label="Open menu"]')
    expect(page).to have_css('nav[data-testid="mobile-bottom-nav"]')
    expect(page).to have_css(%(a[aria-label="Dashboard"][aria-current="page"]))
    expect(page).to have_css(%(a[aria-label="Inventory"][href="#{medications_path}"]))
    expect(page).to have_css(%(a[aria-label="Reports"][href="#{reports_path}"]))
    expect(page).to have_css(%(a[aria-label="Profile"][href="#{profile_path}"]))
    expect(page).to have_no_css(%(a[aria-label="Locations"][href="#{locations_path}"]))
    expect(page).to have_no_css('nav.mobile-nav')
  end

  scenario 'marks Dashboard active on the dashboard route' do
    page.current_window.resize_to(375, 667)
    visit dashboard_path

    expect(page).to have_css(%(a[aria-label="Dashboard"][aria-current="page"]))
  end

  scenario 'uses one navigation system at the md breakpoint' do
    page.current_window.resize_to(767, 844)
    visit root_path

    expect(navigation_visibility).to include(
      'bottom_nav' => true,
      'sidebar' => false,
      'header' => true,
      'fab' => false
    )

    page.current_window.resize_to(768, 844)

    expect(navigation_visibility).to include(
      'bottom_nav' => false,
      'sidebar' => true,
      'header' => false,
      'fab' => false
    )
  end

  scenario 'keeps the medicine-first dashboard readable above the bottom navigation' do
    page.current_window.resize_to(390, 844)
    visit dashboard_path

    expect(page).to have_css('[data-testid="dashboard-daily-summary"]')
    expect(page).to have_css('[data-testid="dashboard-medicine-list"]')
    expect(page).to have_text('Today:')
    expect(page).to have_text('Today’s medicines')
    expect(page).to have_no_text('Compliance')
    expect(page).to have_no_text('Next Dose')
    expect(page).to have_no_css('button[aria-label="Open quick actions"]')

    main_left = page.evaluate_script('document.querySelector("main").getBoundingClientRect().left')
    expect(main_left).to eq(0)

    nav_top = page.evaluate_script(
      'document.querySelector("[data-testid=\"mobile-bottom-nav\"]").getBoundingClientRect().top'
    )
    summary_bottom = page.evaluate_script(
      'document.querySelector("[data-testid=\"dashboard-daily-summary\"]").getBoundingClientRect().bottom'
    )
    expect(summary_bottom).to be < nav_top
  end

  scenario 'opens a left-side drawer with navigation and accessible sizing' do
    page.current_window.resize_to(375, 667)
    visit root_path

    find('button[aria-label="Open menu"]').click

    within('[role="dialog"]') do
      expect(page).to have_link('Inventory', href: medications_path)
      expect(page).to have_link('Locations', href: locations_path)
      expect(page).to have_link('People', href: people_path)
      expect(page).to have_link('Medication Finder', href: medication_finder_path)
      expect(page).to have_link('Reports', href: reports_path)
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

  scenario 'opens and closes the floating action menu and launches the medication workflow' do
    page.current_window.resize_to(375, 667)
    visit medications_path

    expect(page).to have_css('button[aria-label="Open quick actions"]')

    find('button[aria-label="Open quick actions"]').click
    expect(page).to have_css('button[aria-label="Close quick actions"]')
    expect(page).to have_css('[data-testid="floating-action-menu-items"]', visible: :visible)
    expect(page).to have_link('Add Medication', href: add_medication_path)

    find('body').send_keys(:escape)
    expect(page).to have_css('button[aria-label="Open quick actions"]')
    expect(page).to have_no_css('[data-testid="floating-action-menu-items"]', visible: :visible)

    find('button[aria-label="Open quick actions"]').click
    find('[data-testid="floating-action-backdrop"]').click
    expect(page).to have_css('button[aria-label="Open quick actions"]')

    find('button[aria-label="Open quick actions"]').click
    click_link 'Add Medication'

    expect(page).to have_css('button[aria-label="Open quick actions"]', wait: 10)
    expect(page).to have_text('Who is this medication for?', wait: 10)
  end

  def navigation_visibility
    page.evaluate_script(navigation_visibility_script)
  end
end
