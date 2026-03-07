# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Session expiry handling' do
  fixtures :accounts, :people, :users

  let(:user) { users(:jane) }

  it 'redirects stale protected form submissions back to login instead of rendering 422' do
    sign_in(user)

    original_value = ActionController::Base.allow_forgery_protection
    ActionController::Base.allow_forgery_protection = true

    patch profile_path, params: { person: { date_of_birth: 30.years.ago.to_date } }

    expect(response).to redirect_to('/login')
    expect(response).to have_http_status(:see_other)

    follow_redirect!

    expect(response.body).to include('Your session expired. Please sign in again.')
  ensure
    ActionController::Base.allow_forgery_protection = original_value
  end
end
