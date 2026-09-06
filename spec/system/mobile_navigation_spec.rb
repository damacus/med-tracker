# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Mobile Navigation', :browser do
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
        const expectedLabels = ['Next Due', 'Due Now', 'Tasks Left']
        const labels = Array.from(document.querySelectorAll('#main-content p'))

        return expectedLabels.map((expectedLabel) => {
          const label = labels.find((element) => element.textContent.trim() === expectedLabel)

          return {
            selector: label ? (label.id ? `#${label.id}` : label.tagName.toLowerCase()) : 'p',
            found: Boolean(label),
            clientWidth: label ? label.clientWidth : 0,
            scrollWidth: label ? label.scrollWidth : 0,
            overflows: !label || label.scrollWidth > label.clientWidth + 1,
            viewport: { width: window.innerWidth, height: window.innerHeight }
          }
        })
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
    within('[data-testid="mobile-rail"]') do
      expect(page).to have_link('Home', href: dashboard_path)
      expect(page).to have_link('Inventory', href: medications_path)
      expect(page).to have_link('Medicine Finder', href: medication_finder_path)
      expect(page).to have_css('a', count: 3)
      expect(page).to have_css('a[aria-label="Home"][aria-current="page"]')
    end
    expect(page).to have_no_css('nav.mobile-nav')
  end

  scenario 'marks Home active on the dashboard route' do
    page.current_window.resize_to(375, 667)
    visit dashboard_path

    expect(page).to have_css(%(a[aria-label="Home"][aria-current="page"]))
  end

  scenario 'chooses and reorders shortcuts from profile settings and retains them after reload' do
    page.current_window.resize_to(320, 844)
    visit profile_path
    within('[data-testid="profile-mobile-shortcuts-sheet"]') do
      click_button 'Bottom bar shortcuts'
    end
    select 'People', from: 'Shortcut 1'
    select 'Reports', from: 'Shortcut 2'
    select 'Profile', from: 'Shortcut 3'
    click_button 'Save shortcuts'

    within('[data-testid="mobile-rail"]') do
      expect(page).to have_link('People')
      expect(all('a').map(&:text)).to eq(%w[People Reports Profile])
      click_link 'People'
    end
    expect(page).to have_current_path(people_path)
    page.refresh
    within('[data-testid="mobile-rail"]') do
      expect(all('a').map(&:text)).to eq(%w[People Reports Profile])
      expect(page).to have_css('a[aria-label="People"][aria-current="page"]')
    end
  end

  scenario 'keeps labelled quick links inside a narrow phone viewport' do
    page.current_window.resize_to(320, 844)
    visit dashboard_path
    page.evaluate_script('document.fonts.ready.then(() => true)')

    within('[data-testid="mobile-rail"]') do
      expect(all('a').map(&:text)).to eq(['Home', 'Inventory', 'Medicine Finder'])
      find('a[aria-label="Medicine Finder"] span').execute_script("this.innerHTML = 'Medicine<br>Finder'")
      icon_tops = all('a svg').map { |icon| icon.evaluate_script('this.getBoundingClientRect().top') }
      expect(icon_tops.max - icon_tops.min).to be <= 1
      all('a').each do |link|
        bounds = link.evaluate_script('this.getBoundingClientRect().toJSON()')
        expect(bounds['width']).to be >= 44
        expect(bounds['height']).to be >= 44
        expect(bounds['left']).to be >= 0
        expect(bounds['right']).to be <= 320
        expect(link.evaluate_script('this.scrollWidth <= this.clientWidth')).to be(true)
      end
      click_link 'Inventory'
    end

    within('[data-testid="mobile-rail"]') do
      expect(page).to have_css('a[aria-label="Inventory"][aria-current="page"]')
      click_link 'Medicine Finder'
    end

    expect(page).to have_current_path(medication_finder_path)
    within('[data-testid="mobile-rail"]') do
      expect(page).to have_css('a[aria-label="Medicine Finder"][aria-current="page"]')
    end
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

  scenario 'keeps translated quick-link labels inside their touch targets' do
    allow(I18n).to receive(:locale).and_return(:cy)
    page.current_window.resize_to(320, 844)
    visit dashboard_path

    within('[data-testid="mobile-rail"]') do
      expect(page).to have_link(I18n.t('layouts.mobile_rail.finder'))
      icon_tops = all('a svg').map { |icon| icon.evaluate_script('this.getBoundingClientRect().top') }
      expect(icon_tops.max - icon_tops.min).to be <= 1
      all('a').each do |link|
        expect(link.evaluate_script('this.scrollWidth <= this.clientWidth')).to be(true)
        expect(link.evaluate_script('this.scrollHeight <= this.clientHeight')).to be(true)
      end
    end
  end

  scenario 'keeps core mobile dashboard metric labels readable beside the rail' do
    page.current_window.resize_to(390, 844)
    visit dashboard_path

    overflowing_labels = mobile_metric_label_overflow.select { |label| label['overflows'] }

    expect(overflowing_labels).to be_empty
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
    expect(drawer[:'aria-label']).to eq(I18n.t('ruby_ui.common.navigation_menu'))

    within('[role="dialog"]') do
      all('a, button').each do |target|
        height = target.evaluate_script('this.getBoundingClientRect().height')
        expect(height).to be >= 24,
                          "target_failures=[{selector:#{target[:id].presence || target.tag_name}," \
                          "role:#{target[:role]},height:#{height},viewport:{width:375,height:667}}]"
      end
    end
  end

  scenario 'dismisses the drawer with backdrop and Escape and can reopen' do
    page.current_window.resize_to(375, 667)
    visit root_path

    open_mobile_menu

    expect(find('body')[:class]).to include('overflow-hidden')

    page.execute_script("document.querySelector('[data-testid=\"drawer-backdrop\"]').click()")
    expect(page).to have_no_css('[role="dialog"]')
    expect(find('body')[:class]).not_to include('overflow-hidden')
    expect(page.evaluate_script('document.activeElement.getAttribute("aria-label")')).to eq('Open menu')

    open_mobile_menu

    find('body').send_keys(:escape)
    expect(page).to have_no_css('[role="dialog"]')

    open_mobile_menu
  end

  scenario 'keeps keyboard focus inside the navigation sheet' do
    page.current_window.resize_to(375, 667)
    visit root_path

    open_mobile_menu
    drawer = find('[role="dialog"]')
    tabbable_count = drawer.all(
      'a[href], button:not([disabled]), input:not([disabled]):not([type="hidden"]), ' \
      'select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])'
    ).count

    expect(tabbable_count).to be_positive

    (tabbable_count + 1).times do
      page.active_element.send_keys(:tab)
      expect(page.evaluate_script("document.activeElement.closest('[role=dialog]') !== null")).to be(true)
    end

    (tabbable_count + 1).times do
      page.active_element.send_keys(%i[shift tab])
      expect(page.evaluate_script("document.activeElement.closest('[role=dialog]') !== null")).to be(true)
    end
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

  def open_mobile_menu
    find('button[aria-label="Open menu"]').click
    expect(page).to have_css('[role="dialog"]', wait: 5)
  rescue RSpec::Expectations::ExpectationNotMetError
    find('button[aria-label="Open menu"]').click
    expect(page).to have_css('[role="dialog"]', wait: 5)
  end
end
