# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Dashboard person overflow menu', :browser do
  fixtures :accounts, :people, :users, :locations, :medications, :dosages, :schedules

  before do
    driven_by(:playwright)
    sign_in(users(:admin))
  end

  it 'closes More people with Escape and restores focus to its trigger' do
    visit dashboard_path

    trigger = find('[data-testid="dashboard-person-overflow"] button')
    trigger.click
    expect(page).to have_css('[data-testid="dashboard-person-overflow"] [role="menuitem"]', visible: :visible)

    find('body').send_keys(:escape)

    expect(page).to have_no_css('[data-testid="dashboard-person-overflow"] [role="menuitem"]', visible: :visible)
    expect(page.evaluate_script('document.activeElement.getAttribute("aria-label")')).to eq('More people')
  end
end
