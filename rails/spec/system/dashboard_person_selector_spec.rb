# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Dashboard person selector', :browser do
  fixtures :accounts, :people, :users, :locations, :medications, :dosages, :schedules

  before do
    driven_by(:playwright)
    sign_in(users(:admin))
  end

  it 'expands the family card and selects all family through Turbo navigation' do
    visit dashboard_path

    selector = find('[data-testid="dashboard-person-selector"]')
    expect(selector).to have_css('details:not([open])')

    selector.find('summary').click
    expect(selector).to have_css('details[open]')

    selector.find('[role="radio"]', text: 'All Family').click

    expect(page).to have_current_path(
      dashboard_path(dashboard_person_id: DashboardPresenter::ALL_FAMILY_PERSON_ID)
    )
    expect(page).to have_css(
      '[data-testid="dashboard-person-selector"] [role="radio"][aria-checked="true"]' \
      '[aria-label="All Family"]',
      visible: :all
    )
    expect(page).to have_css('[data-testid="dashboard-person-selector"] details:not([open])')
  end

  it 'supports disclosure and selection from the keyboard' do
    visit dashboard_path

    summary = find('[data-testid="dashboard-person-selector-summary"]')
    summary.send_keys(:enter)
    expect(page).to have_css('[data-testid="dashboard-person-selector-disclosure"][open]')

    summary.send_keys(:space)
    expect(page).to have_css('[data-testid="dashboard-person-selector-disclosure"]:not([open])')

    summary.send_keys(:enter)
    selected = find('[role="radio"][aria-checked="true"]')
    selected.send_keys(:right)
    focused_value = page.evaluate_script('document.activeElement.dataset.value')
    page.send_keys(:enter)

    expect(page).to have_current_path(dashboard_path(dashboard_person_id: focused_value))
  end

  it 'keeps the selector and summary metrics within mobile and desktop viewports' do
    [390, 1280].each do |width|
      page.current_window.resize_to(width, 844)
      visit dashboard_path

      find('[data-testid="dashboard-person-selector-summary"]').click
      metric_tops = page.evaluate_script(<<~JAVASCRIPT)
        Array.from(document.querySelectorAll('[data-testid="dashboard-metrics"] > *'))
          .map((card) => Math.round(card.getBoundingClientRect().top))
      JAVASCRIPT

      expect(metric_tops.uniq.one?).to be(true)
      expect(page.evaluate_script('document.documentElement.scrollWidth')).to be <= width
    end
  end
end
