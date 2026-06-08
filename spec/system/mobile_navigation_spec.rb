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

        const rail = document.querySelector('[data-testid="mobile-rail"]')
        const sidebar = Array.from(document.querySelectorAll('aside')).find((element) => element !== rail)

        return {
          rail: visible(rail),
          sidebar: visible(sidebar),
          header: visible(document.querySelector('header')),
          fab: Boolean(document.querySelector('[data-testid="floating-action-menu-toggle"]'))
        }
      })()
    JS
  end
  let(:mobile_metric_label_overflow_script) do
    <<~JS
      (() => {
        const expectedLabels = ['People', 'Active Schedules', 'Next Dose']
        const labels = Array.from(document.querySelectorAll('#main-content p'))

        return expectedLabels.map((text) => {
          const label = labels.find((element) => element.textContent.trim() === text)

          return {
            text,
            found: Boolean(label),
            clientWidth: label ? label.clientWidth : 0,
            scrollWidth: label ? label.scrollWidth : 0,
            overflows: !label || label.scrollWidth > label.clientWidth + 1
          }
        })
      })()
    JS
  end
  let(:mobile_shell_overlap_script) do
    <<~JS
      (() => {
        const rail = document.querySelector('[data-testid="mobile-rail"]')
        const content = document.querySelector('#main-content')
        const railBounds = rail.getBoundingClientRect()
        const contentBounds = content.getBoundingClientRect()

        return {
          railRight: railBounds.right,
          contentLeft: contentBounds.left,
          overlaps: contentBounds.left < railBounds.right
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
    expect(page).to have_css('aside[data-testid="mobile-rail"]')
    expect(page).to have_css(%(a[aria-label="Dashboard"][aria-current="page"]))
    expect(page).to have_css(%(a[aria-label="Inventory"][href="#{medications_path}"]))
    expect(page).to have_css(%(a[aria-label="Locations"][href="#{locations_path}"]))
    expect(page).to have_css(%(a[aria-label="Reports"][href="#{reports_path}"]))
    expect(page).to have_css(%(a[aria-label="Profile"][href="#{profile_path}"]))
    expect(page).to have_no_css('nav.mobile-nav')
  end

  scenario 'marks Dashboard active on the dashboard route' do
    page.current_window.resize_to(375, 667)
    visit dashboard_path

    expect(page).to have_css(%(a[aria-label="Dashboard"][aria-current="page"]))
  end

  scenario 'uses one navigation system at the md breakpoint' do
    page.current_window.resize_to(760, 844)
    visit root_path

    expect(navigation_visibility).to include(
      'rail' => true,
      'sidebar' => false,
      'header' => true,
      'fab' => false
    )

    page.current_window.resize_to(768, 844)

    expect(navigation_visibility).to include(
      'rail' => false,
      'sidebar' => true,
      'header' => false,
      'fab' => false
    )
  end

  scenario 'keeps core mobile dashboard metric labels readable beside the rail' do
    page.current_window.resize_to(390, 844)
    visit dashboard_path

    overflowing_labels = mobile_metric_label_overflow.select { |label| label['overflows'] }

    expect(overflowing_labels).to be_empty
  end

  scenario 'keeps dashboard content clear of the mobile rail' do
    page.current_window.resize_to(390, 844)
    visit dashboard_path

    shell_overlap = mobile_shell_overlap
    failure_message = "expected content left #{shell_overlap['contentLeft']} " \
                      "to be >= rail right #{shell_overlap['railRight']}"

    expect(shell_overlap['overlaps']).to be(false), failure_message
  end

  scenario 'opens a left-side drawer with navigation and accessible sizing' do
    page.current_window.resize_to(375, 667)
    visit root_path

    open_mobile_menu

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

    open_mobile_menu

    page.execute_script("document.querySelector('[data-testid=\"drawer-backdrop\"]').click()")
    expect(page).to have_no_css('[role="dialog"]')

    open_mobile_menu

    find('body').send_keys(:escape)
    expect(page).to have_no_css('[role="dialog"]')

    open_mobile_menu
  end

  scenario 'does not render the floating action menu on mobile' do
    page.current_window.resize_to(375, 667)
    visit root_path

    expect(page).to have_css('aside[data-testid="mobile-rail"]')
    expect(page).to have_no_css('[data-testid="floating-action-menu-toggle"]')
    expect(page).to have_no_css('[data-testid="floating-action-menu-items"]')
  end

  def navigation_visibility
    page.evaluate_script(navigation_visibility_script)
  end

  def mobile_metric_label_overflow
    page.evaluate_script(mobile_metric_label_overflow_script)
  end

  def mobile_shell_overlap
    page.evaluate_script(mobile_shell_overlap_script)
  end

  def open_mobile_menu
    find('button[aria-label="Open menu"]').click
    expect(page).to have_css('[role="dialog"]', wait: 5)
  rescue RSpec::Expectations::ExpectationNotMetError
    find('button[aria-label="Open menu"]').click
    expect(page).to have_css('[role="dialog"]', wait: 5)
  end
end
