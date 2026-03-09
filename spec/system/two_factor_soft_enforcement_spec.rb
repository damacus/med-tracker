# frozen_string_literal: true

require 'rails_helper'

# rubocop:disable Capybara/NegationMatcherAfterVisit
RSpec.describe 'Two-Factor Soft Enforcement' do
  fixtures :accounts, :people, :users

  it 'shows the privileged-user reminder on the profile page in the browser' do
    user = users(:damacus)
    clear_2fa_for_account(user.person.account)

    login_as(user)
    visit profile_path

    expect(page).to have_content('For enhanced security, please set up two-factor authentication')
    expect(page).to have_current_path(profile_path)
  end
end
# rubocop:enable Capybara/NegationMatcherAfterVisit
