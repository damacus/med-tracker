# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Profile data exports' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages, :schedules

  before { sign_in(users(:admin)) }

  it 'shows the self-service data backup entry point and warning' do
    get profile_path

    expect(response).to have_http_status(:ok)
    expect(response.body).to include('Data backup')
    expect(response.body).to include('Unencrypted ZIP exports are not password protected')
  end

  it 'downloads a health-data JSON export from the profile page' do
    get profile_data_export_path('health_data_json')

    expect(response).to have_http_status(:ok)
    expect(response.media_type).to eq('application/json')
    expect(response.parsed_body.fetch('format')).to eq('medtracker.health_data.v1')
  end

  it 'downloads a backup ZIP without sensitive platform records from the profile page' do
    get profile_data_export_path('backup_zip')

    expect(response).to have_http_status(:ok)
    expect(response.media_type).to eq('application/zip')
    expect(response.headers.fetch('Content-Disposition')).to include('medtracker-backup-')
    expect(response.body.byteslice(0, 2)).to eq('PK')
    expect(response.body).not_to include('api_sessions', 'api_app_tokens', 'native_device_tokens',
                                         'push_subscriptions', 'security_audit_events')
  end
end
