# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'GET /medication-finder' do
  fixtures :accounts, :people, :users

  def login_as_admin
    post '/login', params: { email: accounts(:john_doe).email, password: 'password' }
  end

  it 'allows camera access via the global Permissions-Policy header' do
    login_as_admin

    get medication_finder_path

    expect(response).to have_http_status(:ok)
    expect(response.headers['Permissions-Policy']).to eq('geolocation=(), camera=(self), microphone=()')
  end

  it 'does not set a per-action Permissions-Policy override' do
    login_as_admin

    get medication_finder_path

    expect(response.headers['Permissions-Policy']).not_to include('camera=()')
  end
end
