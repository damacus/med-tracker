# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'API v1 data exports' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages, :schedules

  let(:user) { users(:admin) }
  let(:login_data) { api_login(user) }
  let(:household_id) { login_data.dig('household', 'id') }
  let(:headers) { api_auth_headers(login_data.fetch('access_token')) }

  it 'returns health-data JSON exports through the shared export service' do
    expect do
      get api_v1_household_data_export_path(household_id, 'health_data_json'), headers: headers, as: :json
    end.to change(export_audit_events, :count).by(1)

    expect(response).to have_http_status(:ok)
    expect(response.headers.fetch('Cache-Control')).to include('no-store')
    expect(response.parsed_body.dig('data', 'format')).to eq('medtracker.health_data.v1')
    expect(response.parsed_body.dig('data', 'records')).to include('people', 'medications')
    expect(export_audit_events.last.metadata).to include(
      'encrypted' => false,
      'export_mode' => 'health_data_json',
      'record_counts' => be_present
    )
  end

  it 'returns unencrypted backup ZIP payloads without token/session records' do
    expect do
      get api_v1_household_data_export_path(household_id, 'backup_zip'), headers: headers, as: :json
    end.to change(export_audit_events, :count).by(1)

    expect(response).to have_http_status(:ok)
    expect(response.headers.fetch('Cache-Control')).to include('no-store')
    data = response.parsed_body.fetch('data')
    expect(data.fetch('content_type')).to eq('application/zip')
    expect(data.fetch('filename')).to end_with('.zip')
    expect(Base64.strict_decode64(data.fetch('base64')).byteslice(0, 2)).to eq('PK')
    expect(export_audit_events.last.metadata).to include(
      'encrypted' => false,
      'export_mode' => 'backup_zip',
      'record_counts' => be_present
    )
  end

  it 'requires a passphrase for encrypted migration bundles' do
    get api_v1_household_data_export_path(household_id, 'encrypted_migration_bundle'), headers: headers, as: :json

    expect(response).to have_http_status(:unprocessable_content)
    expect(response.headers.fetch('Cache-Control')).to include('no-store')

    expect do
      get api_v1_household_data_export_path(household_id, 'encrypted_migration_bundle'),
          headers: headers.merge('X-MedTracker-Portable-Passphrase' => 'correct horse battery staple'),
          as: :json
    end.to change(export_audit_events, :count).by(1)

    expect(response).to have_http_status(:ok)
    expect(response.headers.fetch('Cache-Control')).to include('no-store')
    expect(response.parsed_body.dig('data', 'format')).to eq('medtracker.portable.encrypted.v1')
    expect(export_audit_events.last.metadata).to include(
      'encrypted' => true,
      'export_mode' => 'encrypted_migration_bundle',
      'record_counts' => be_present
    )
  end

  def export_audit_events
    SecurityAuditEvent.where(event_type: 'portable_data.exported')
  end
end
