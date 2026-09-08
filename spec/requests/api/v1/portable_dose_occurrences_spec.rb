require 'rails_helper'

RSpec.describe 'API v1 portable dose outcome export' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages, :schedules

  let(:login_data) { api_login(users(:admin)) }
  let(:household_id) { login_data.dig('household', 'id') }
  let(:passphrase) { 'portable outcome phrase' }
  let(:headers) do
    api_auth_headers(login_data.fetch('access_token')).merge('X-MedTracker-Portable-Passphrase' => passphrase)
  end

  it 'returns an encrypted v2 bundle when explicitly requested' do
    headers
    source = schedules(:john_movicol).reload
    record = source.medication_dose_occurrences.create!(
      window_starts_on: Date.current, position: 1, outcome: 'not_taken', note: 'Resting',
      resolved_at: Time.current, resolved_by_membership: users(:admin).person.account.first_active_household_membership
    )
    get "/api/v1/households/#{household_id}/portable_export", params: { portable_format: 'medtracker.portable.v2' },
                                                              headers: headers
    expect(response).to have_http_status(:ok)
    payload = PortableData::Encryptor.decrypt(response.parsed_body.fetch('data'), passphrase: passphrase)
    expect(payload.fetch('format')).to eq('medtracker.portable.v2')
    expect(payload.dig('records', 'dose_occurrences').sole.fetch('portable_id')).to eq(record.portable_id)
    expect(response.body).not_to include('Resting')
  end

  it 'rejects unsupported formats without echoing supplied text' do
    get "/api/v1/households/#{household_id}/portable_export", params: { portable_format: 'private clinical text' },
                                                              headers: headers
    expect(response).to have_http_status(:unprocessable_content)
    expect(response.body).not_to include('private clinical text')
  end
end
