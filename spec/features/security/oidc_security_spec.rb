# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'OIDC Login page without credentials', type: :system do
  fixtures :accounts, :people, :users

  it 'does not show OIDC button when credentials are not configured' do
    visit login_path
    expect(page).to have_content('Welcome back')
    expect(page).to have_no_button(/Continue with/i)
  end
end
