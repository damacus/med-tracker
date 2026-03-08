# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Session expiry', :js do
  fixtures :accounts, :people, :users

  let(:user) { users(:damacus) }

  it 'redirects to login when a turbo form submission hits an expired session' do
    login_as(user)
    visit profile_path

    page.driver.with_playwright_page do |playwright_page|
      playwright_page.context.clear_cookies
    end

    click_button 'Save'

    expect(page).to have_current_path('/login')
    expect(page).to have_button('Sign In to Dashboard')
  end
end
