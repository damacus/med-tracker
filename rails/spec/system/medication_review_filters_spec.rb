# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Medication review filters', :browser do
  fixtures :accounts, :people, :users

  before do
    driven_by(:playwright)
    sign_in(users(:admin))
  end

  it 'keeps server-selected review links active when moved with the keyboard' do
    page.current_window.resize_to(390, 844)
    visit medication_review_prompts_path(household_slug: browser_household.slug)

    active_link = find('[data-testid="review-status-tabs"] a[data-state="active"]')
    next_link = find('[data-testid="review-status-tabs"] a[data-value="reviewed"]')
    active_path = page.current_url

    active_link.send_keys(:right)

    expect(page).to have_current_path(active_path)
    expect(active_link['data-state']).to eq('active')
    expect(active_link['aria-current']).to eq('page')
    expect(next_link['data-state']).to eq('inactive')
    expect(next_link['aria-selected']).to eq('false')
  end
end
