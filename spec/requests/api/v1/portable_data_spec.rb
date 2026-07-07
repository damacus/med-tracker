# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'API v1 portable data' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages, :schedules

  let(:user) { users(:admin) }
  let(:login_data) { api_login(user) }
  let(:household_id) { login_data.dig('household', 'id') }
  let(:headers) { api_auth_headers(login_data.fetch('access_token')) }
  let(:portable_import_payload) do
    {
      format: 'medtracker.portable.v1',
      scope: 'single_person',
      exported_at: Time.current.iso8601,
      source_instance_id: 'mobile:request-spec',
      records: {
        people: [
          {
            portable_id: 'request-person-portable-1',
            name: 'Request Import User',
            date_of_birth: '1990-01-01',
            person_type: 'adult',
            has_capacity: true,
            location_portable_ids: ['request-location-portable-1']
          }
        ],
        locations: [
          { portable_id: 'request-location-portable-1', name: 'Request Home' }
        ]
      }
    }
  end

  def passphrase
    'correct horse battery staple'
  end

  def encrypted_import_payload
    PortableData::Encryptor.encrypt(portable_import_payload, passphrase: passphrase)
  end

  def portable_headers(headers = self.headers)
    headers.merge('X-MedTracker-Portable-Passphrase' => passphrase)
  end

  def add_backup_owner(household)
    account = Account.create!(
      email: "portable-backup-owner-#{SecureRandom.hex(4)}@example.test",
      status: :verified
    )
    household.household_memberships.create!(account: account, role: :owner, status: :active)
  end

  it 'requires bearer auth for portable export' do
    get "/api/v1/households/#{household_id}/portable_export",
        headers: portable_headers({}),
        as: :json

    expect(response).to have_http_status(:unauthorized)
  end

  it 'returns an encrypted portable export for the current household' do
    get "/api/v1/households/#{household_id}/portable_export",
        headers: portable_headers,
        as: :json

    expect(response).to have_http_status(:ok)
    envelope = response.parsed_body.fetch('data')
    payload = PortableData::Encryptor.decrypt(envelope, passphrase: passphrase)

    expect(envelope.fetch('format')).to eq('medtracker.portable.encrypted.v1')
    expect(response.body).not_to include(medications(:paracetamol).name)
    expect(payload.dig('records', 'people')).not_to be_empty
    expect(payload.dig('records', 'people').first).to include('portable_id')
  end

  it 'rejects portable export passphrases sent in query params' do
    get "/api/v1/households/#{household_id}/portable_export",
        params: { passphrase: passphrase },
        headers: headers,
        as: :json

    expect(response).to have_http_status(:unprocessable_content)
    expect(response.parsed_body.dig('error', 'message')).to eq('Portable passphrase header is required')
  end

  it 'rejects portable imports without passphrase headers' do
    post "/api/v1/households/#{household_id}/portable_imports/dry_run",
         params: { bundle: encrypted_import_payload },
         headers: headers,
         as: :json

    expect(response).to have_http_status(:unprocessable_content)

    post "/api/v1/households/#{household_id}/portable_imports",
         params: { bundle: encrypted_import_payload },
         headers: headers,
         as: :json

    expect(response).to have_http_status(:unprocessable_content)
  end

  it 'returns a portable mobile snapshot with portable relationship fields' do
    request_headers = headers

    expect do
      get "/api/v1/households/#{household_id}/mobile_snapshot",
          headers: request_headers,
          as: :json
    end.to change(SecurityAuditEvent, :count).by(1)

    expect(response).to have_http_status(:ok)
    snapshot = response.parsed_body.fetch('data')
    medication = snapshot.dig('records', 'medications').first
    audit_event = SecurityAuditEvent.order(:created_at).last

    expect(snapshot).to include('format' => 'medtracker.portable.v1')
    expect(snapshot.dig('records', 'people').first).to include('portable_id')
    expect(medication).to include('portable_id', 'location_portable_id')
    expect(medication).not_to include('id', 'location_id')
    expect(audit_event).to have_attributes(
      household_id: household_id,
      actor_account_id: user.person.account_id,
      event_type: 'portable_data.mobile_snapshot_read'
    )
    expect(audit_event.metadata).to include('encrypted' => false, 'record_counts' => include('people'))
  end

  it 'dry-runs encrypted portable imports without writing records' do
    expect do
      post "/api/v1/households/#{household_id}/portable_imports/dry_run",
           params: { bundle: encrypted_import_payload },
           headers: portable_headers,
           as: :json
    end.not_to change(Person, :count)

    expect(response).to have_http_status(:ok)
    expect(response.parsed_body.dig('data', 'applied')).to be(false)
    expect(response.parsed_body.dig('data', 'counts')).to include('people' => 1, 'locations' => 1)
  end

  it 'applies encrypted portable imports with app token bearer auth' do
    api_session = ApiSession.lookup_by_access_token(login_data.fetch('access_token'))
    membership = api_session.household_membership
    _token, raw_token = ApiAppToken.issue_for(
      account: user.person.account,
      household_membership: membership,
      name: 'RSpec mobile import'
    )
    before_counts = [Person.count, Location.count]

    post "/api/v1/households/#{household_id}/portable_imports",
         params: { bundle: encrypted_import_payload },
         headers: portable_headers(api_auth_headers(raw_token)),
         as: :json

    expect(response).to have_http_status(:created)
    expect([Person.count, Location.count]).to eq(before_counts.map { |count| count + 1 })
    expect(response.parsed_body.dig('data', 'applied')).to be(true)
    expect(Person.find_by!(portable_id: 'request-person-portable-1').household_id).to eq(household_id)
  end

  it 'rejects portable endpoints for the wrong household' do
    other_household = create(:household)

    get "/api/v1/households/#{other_household.id}/mobile_snapshot",
        headers: headers,
        as: :json

    expect(response).to have_http_status(:forbidden)
  end

  it 'rejects portable endpoints while the account is locked' do
    request_headers = headers
    AccountLockout.create!(
      account_id: user.person.account.id,
      key: SecureRandom.hex(16),
      deadline: 30.minutes.from_now
    )

    get "/api/v1/households/#{household_id}/mobile_snapshot",
        headers: request_headers,
        as: :json

    expect(response).to have_http_status(:unauthorized)
  end

  it 'rejects portable endpoints after membership revocation' do
    request_headers = headers
    membership = user.person.account.first_active_household_membership
    add_backup_owner(membership.household)
    membership.update!(status: :revoked)

    get "/api/v1/households/#{household_id}/mobile_snapshot",
        headers: request_headers,
        as: :json

    expect(response).to have_http_status(:unauthorized)
  end

  it 'rejects portable endpoints after permissions versions change' do
    request_headers = headers
    membership = user.person.account.first_active_household_membership
    membership.update!(permissions_version: membership.permissions_version + 1)

    get "/api/v1/households/#{household_id}/mobile_snapshot",
        headers: request_headers,
        as: :json

    expect(response).to have_http_status(:unauthorized)
  end

  it 'rejects portable imports that contain Rails numeric IDs' do
    payload = portable_import_payload.deep_dup
    payload[:records][:people].first[:id] = people(:admin).id

    post "/api/v1/households/#{household_id}/portable_imports",
         params: { bundle: PortableData::Encryptor.encrypt(payload, passphrase: passphrase) },
         headers: portable_headers,
         as: :json

    expect(response).to have_http_status(:unprocessable_content)
    expect(response.parsed_body.dig('data', 'errors').join).to include('Rails numeric IDs')
    expect(Person.exists?(portable_id: 'request-person-portable-1')).to be(false)
  end

  it 'filters portable passphrase parameters from logs if a client sends them' do
    filtered = ActiveSupport::ParameterFilter
               .new(Rails.application.config.filter_parameters)
               .filter(passphrase: passphrase)

    expect(filtered.fetch(:passphrase)).to eq('[FILTERED]')
  end
end
