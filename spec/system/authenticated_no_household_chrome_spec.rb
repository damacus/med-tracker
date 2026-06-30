# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Authenticated no-household chrome' do
  fixtures :accounts, :people, :users

  it 'renders the login page without authenticated mobile chrome after a no-household redirect', :js do
    page.current_window.resize_to(390, 844)

    visit '/login'
    fill_in 'Email address', with: users(:damacus).email_address
    fill_in 'Password', with: 'password'
    click_button 'Sign In to Dashboard'

    expect(page).to have_current_path('/login')
    expect(page).to have_no_css('[data-testid="mobile-rail"]')
    expect(page).to have_no_css('[data-responsive-shell-role="sidebar"]')
    expect(main_left_offset).to eq(0)
  end

  def main_left_offset
    page.evaluate_script(<<~JS)
      Math.round(document.querySelector('main').getBoundingClientRect().left)
    JS
  end
end
