# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Session expiry handling' do
  fixtures :accounts, :people, :users

  let(:user) { users(:jane) }

  it 'preserves an active login older than a day and updates its last use' do
    sign_in(user)
    key = user.person.account.account_active_session_keys.sole
    key.update!(created_at: 60.days.ago, last_use: 2.days.ago)

    get profile_path

    expect(response).to have_http_status(:ok)
    expect(key.reload.last_use).to be_within(5.seconds).of(Time.current)
  end

  it 'requires login after the default inactivity period' do
    sign_in(user)
    user.person.account.account_active_session_keys.sole.update!(last_use: 31.days.ago)

    get profile_path

    expect(response).to redirect_to('/login')
  end

  it 'uses the configured inactivity period on protected requests' do
    allow(ENV).to receive(:fetch).and_call_original
    allow(ENV).to receive(:fetch).with('SESSION_INACTIVITY_TIMEOUT_DAYS', '30').and_return('7')
    sign_in(user)
    user.person.account.account_active_session_keys.sole.update!(last_use: 8.days.ago)

    get profile_path

    expect(response).to redirect_to('/login')
  end

  it 'honours a configured absolute deadline despite recent activity' do
    allow(ENV).to receive(:fetch).and_call_original
    allow(ENV).to receive(:fetch).with('SESSION_MAX_AGE_DAYS', '0').and_return('90')
    sign_in(user)
    user.person.account.account_active_session_keys.sole.update!(created_at: 91.days.ago)

    get profile_path

    expect(response).to redirect_to('/login')
  end

  it 'restores remembered login after the browser session cookie is lost' do
    sign_in(user)
    post '/remember', params: { remember: 'remember' }
    remembered_cookie = cookies['_remember']
    cookies.delete(Rails.application.config.session_options.fetch(:key))
    cookies['_remember'] = remembered_cookie

    get profile_path

    expect(response).to have_http_status(:ok)
  end

  it 'stores the configured deadline when issuing a remembered cookie' do
    allow(ENV).to receive(:fetch).and_call_original
    allow(ENV).to receive(:fetch).with('SESSION_INACTIVITY_TIMEOUT_DAYS', '30').and_return('7')
    sign_in(user)
    post '/remember', params: { remember: 'remember' }

    deadline = RodauthApp.rodauth.allocate.db[:account_remember_keys]
                         .where(account_id: user.person.account.id).get(:deadline)

    expect(deadline).to be_within(5.seconds).of(7.days.from_now)
  end

  it 'preserves the original age when restoring a remembered session' do
    sign_in(user)
    post '/remember', params: { remember: 'remember' }
    original_login = 20.days.ago.change(usec: 0)
    RodauthApp.rodauth.allocate.db[:account_remember_keys]
              .where(account_id: user.person.account.id).update(created_at: original_login)
    existing_sessions = user.person.account.account_active_session_keys.pluck(:session_id)
    cookies.delete(Rails.application.config.session_options.fetch(:key))

    get profile_path

    key = user.person.account.account_active_session_keys.where.not(session_id: existing_sessions).sole
    expect(key.created_at).to eq(original_login)
  end

  it 'does not restore remembered login beyond a configured absolute deadline' do
    allow(ENV).to receive(:fetch).and_call_original
    allow(ENV).to receive(:fetch).with('SESSION_MAX_AGE_DAYS', '0').and_return('90')
    sign_in(user)
    post '/remember', params: { remember: 'remember' }
    RodauthApp.rodauth.allocate.db[:account_remember_keys]
              .where(account_id: user.person.account.id).update(created_at: 91.days.ago)
    remembered_cookie = cookies['_remember']
    cookies.delete(Rails.application.config.session_options.fetch(:key))
    cookies['_remember'] = remembered_cookie

    get profile_path

    expect(response).to have_http_status(:redirect)
    expect(cookies['_remember']).to be_blank
    expect(response.body).not_to include('Date of birth')
  end

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
