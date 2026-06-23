# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Session expiry', :js do
  fixtures :accounts, :people, :users

  let(:user) { users(:damacus) }

  it 'redirects to login when a turbo form submission hits an expired session' do
    login_as(user)
    visit profile_path

    page.execute_script(<<~JS)
      document.dispatchEvent(new CustomEvent("turbo:before-fetch-response", {
        cancelable: true,
        detail: {
          fetchResponse: {
            response: { url: `${window.location.origin}/login` }
          }
        }
      }))
    JS

    expect(page).to have_current_path('/login')
    expect(page).to have_button('Sign In to Dashboard')
  end
end
