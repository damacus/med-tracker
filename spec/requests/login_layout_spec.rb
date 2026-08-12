# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Login layout' do
  fixtures :accounts, :people, :users

  it 'uses one non-empty CSP nonce across the anonymous response' do
    get login_path

    document = response.parsed_body
    nonce = document.at_css('meta[name="csp-nonce"]')['content']
    activation_script = document.css('script[nonce]').find do |script|
      script.text.include?('applyLoginIllustrations')
    end

    expect(nonce).to be_present
    expect(response.headers.fetch('Content-Security-Policy')).to include("'nonce-#{nonce}'")
    expect(activation_script['nonce']).to eq(nonce)
  end

  it 'generates a new CSP nonce for each anonymous request' do
    get login_path
    first_nonce = response.parsed_body.at_css('meta[name="csp-nonce"]')['content']

    get login_path
    second_nonce = response.parsed_body.at_css('meta[name="csp-nonce"]')['content']

    expect(first_nonce).to be_present
    expect(second_nonce).to be_present
    expect(second_nonce).not_to eq(first_nonce)
  end

  it 'renders without the global mobile navigation chrome' do
    get login_path

    expect(response.body).not_to include('class="nav"')
    expect(response.body).not_to include('nav__brand-link')
  end

  it 'keeps an authenticated owner in the household shell while setting up MFA' do
    user = users(:admin)
    household = ensure_api_household_for(user)
    sign_in(user)

    household_queries = with_runtime_role do
      capture_household_queries { get '/otp-setup' }
    end

    expect(response).to have_http_status(:ok)
    expect(household_queries.length).to eq(1)
    expect(response.body).to include('data-responsive-shell-role="sidebar"')
    expect(response.body).to include('data-testid="mobile-rail"')
    expect(response.body).to include(user.name)
    expect(response.body).to include('Owner')
    expect(response.body).to include('Administration')
    expect(response.body).to include("/households/#{household.slug}/dashboard")
  end

  it 'keeps anonymous password recovery in the auth shell' do
    get '/reset-password-request'

    expect(response).to have_http_status(:ok)
    expect(response.body).not_to include('data-responsive-shell-role="sidebar"')
    expect(response.body).not_to include('data-testid="mobile-rail"')
    expect(response.body).not_to include('class="nav"')
  end

  it 'redirects unauthenticated users to login without routine login-required flash' do
    get dashboard_path

    expect(response).to redirect_to(login_path)
    expect(flash[:alert]).to be_blank
    expect(flash[:notice]).to be_blank

    follow_redirect!

    expect(response.body).not_to include('Please login to continue')
    expect(response.body.scan('role="alert"').count).to eq(0)
  end

  def capture_household_queries(&)
    queries = []
    subscriber = lambda do |_name, _started, _finished, _unique_id, payload|
      next if payload[:cached] || payload[:name] == 'SCHEMA'
      next unless payload[:sql].match?(/household_memberships|households/)

      queries << payload[:sql]
    end

    ActiveSupport::Notifications.subscribed(subscriber, 'sql.active_record', &)
    queries
  end

  def with_runtime_role
    result = nil
    ActiveRecord::Base.connection.transaction(requires_new: true) do
      ActiveRecord::Base.connection.execute('SET LOCAL ROLE med_tracker_app')
      result = yield
      raise ActiveRecord::Rollback
    end
    result
  end
end
