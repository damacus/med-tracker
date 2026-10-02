require 'rails_helper'

RSpec.describe 'API v1 profile' do
  fixtures :all

  let(:user) { users(:jane) }
  let(:login) { api_login(user) }
  let(:headers) { api_auth_headers(login.fetch('access_token')) }
  let(:person) { user.person }
  let(:path) { "/api/v1/households/#{login.dig('household', 'id')}/profile" }

  before { login }

  delegate :account, to: :person

  it 'advertises profile preferences as online operations' do
    get '/api/v1/capabilities', as: :json
    expect(response.parsed_body.dig('data', 'profile')).to include('actions' => %w[show update], 'online_only' => true)
  end

  it 'rejects unauthenticated access and invalid field types' do
    get path, as: :json
    expect(response).to have_http_status(:unauthorized)
    [{ gravatar_enabled: 'false' }, { time_zone: 123 }, { mobile_shortcuts: 'finder' }].each do |attributes|
      patch path, params: { profile: attributes }, headers: headers, as: :json
      expect(response).to have_http_status(:unprocessable_content)
    end
  end

  it 'returns account preferences and the household person with string identifiers' do
    get path, headers: headers, as: :json
    expect(response).to have_http_status(:ok)
    expect(response.headers['Cache-Control']).to include('no-store')
    expect(response.parsed_body.fetch('data')).to include(
      'account_id' => account.id.to_s, 'person_id' => person.id.to_s,
      'date_of_birth' => person.date_of_birth.iso8601,
      'mobile_shortcuts' => account.preferred_mobile_shortcuts
    )
  end

  it 'saves person and account preferences atomically' do
    patch path, params: { profile: { date_of_birth: '1985-06-15', time_zone: 'Europe/London',
                                     gravatar_enabled: true, mobile_shortcuts: %w[profile finder] } },
                headers: headers, as: :json
    expect(response).to have_http_status(:ok)
    expect(person.reload.date_of_birth).to eq(Date.new(1985, 6, 15))
    expect(account.reload.preferred_time_zone).to eq('Europe/London')
    expect(account.gravatar_enabled?).to be(true)
    expect(account.preferred_mobile_shortcuts).to eq(%w[profile finder])
  end

  it 'rolls back the person change when an account preference is invalid' do
    original = person.date_of_birth
    patch path, params: { profile: { date_of_birth: '1985-06-15', time_zone: 'Invalid/Place' } },
                headers: headers, as: :json
    expect(response).to have_http_status(:unprocessable_content)
    expect(person.reload.date_of_birth).to eq(original)
  end

  it 'rejects malformed dates and shortcuts without saving other preferences' do
    [{ date_of_birth: 'not-a-date' }, { mobile_shortcuts: %w[profile profile] }].each do |attributes|
      patch path, params: { profile: attributes.merge(time_zone: 'Europe/Paris') }, headers: headers, as: :json
      expect(response).to have_http_status(:unprocessable_content)
      expect(account.reload.time_zone).not_to eq('Europe/Paris')
    end
  end

  it 'rejects security attributes instead of partially applying the request' do
    patch path, params: { profile: { email: 'other@example.test', role: 'owner', time_zone: 'Europe/Paris' } },
                headers: headers, as: :json
    expect(response).to have_http_status(:unprocessable_content)
    expect(account.reload.email).to eq(user.email_address)
    expect(account.time_zone).not_to eq('Europe/Paris')
  end

  it 'does not use a dependent person as the account profile' do
    dependent = person.household.people.create!(name: 'Another household person', date_of_birth: 30.years.ago.to_date)
    ApiSession.lookup_by_access_token(login.fetch('access_token')).household_membership.update!(person: dependent)
    get path, headers: headers, as: :json
    expect(response).to have_http_status(:not_found)
  end

  it 'checks current person permission before replaying a cached update' do
    replay_headers = headers.merge('Idempotency-Key' => SecureRandom.uuid)
    payload = { profile: { time_zone: 'Europe/London' } }
    patch path, params: payload, headers: replay_headers, as: :json
    expect(response).to have_http_status(:ok)
    patch path, params: payload, headers: replay_headers, as: :json
    expect(response.headers['Idempotency-Replayed']).to eq('true')
    expect(response.headers['Cache-Control']).to include('no-store')
    membership = ApiSession.lookup_by_access_token(login.fetch('access_token')).household_membership
    PersonAccessGrant.where(household_membership: membership, person: person).destroy_all
    patch path, params: payload, headers: replay_headers, as: :json
    expect(response).to have_http_status(:forbidden)
    expect(response.headers['Idempotency-Replayed']).to be_nil
  end
end
