# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'API v1 capabilities' do
  it 'publishes the supported API, auth, portability, sync, and client-tool contracts without auth' do
    get api_v1_capabilities_path, as: :json

    expect(response).to have_http_status(:ok)
    expect(response.headers['Cache-Control']).to include('no-store')

    data = response.parsed_body.fetch('data')
    expect(data).to include(
      'format' => 'medtracker.api.capabilities.v1',
      'api_version' => 'v1',
      'portable_formats' => include('medtracker.portable.v1', 'medtracker.portable.encrypted.v1')
    )
    expect(data.dig('authentication', 'methods')).to include('bearer_session', 'api_app_token')
    expect(data.dig('authentication', 'hosted_mobile')).to eq('oidc_authorization_code_pkce')
    expect(data.dig('authentication', 'oidc_exchange')).to include(
      'supported' => true,
      'pkce_required' => true,
      'household_selection' => true,
      'session_listing' => true,
      'session_revocation' => true
    )
    expect(data).to include('administration' => include('household' => true, 'fresh_mfa_required' => true))
    expect(data.dig('sync', 'portable_ids')).to be(true)
    expect(data.dig('sync', 'numeric_ids')).to eq('backward_compatible')
    expect(data.dig('sync', 'idempotency_keys')).to be(true)
    expect(data.dig('sync', 'etag_conflicts')).to be(true)
    expect(data.dig('sync', 'change_feed')).to be(true)
    expect(data.dig('sync', 'batch_mutations')).to be(true)
    expect(data.dig('sync', 'tombstones')).to be(true)
    expect(data['portable_formats']).to include('medtracker.portable.v2')
    expect(data).to include(
      'backups' => include('unencrypted_zip' => true, 'health_data_json' => true),
      'fhir' => include(
        'version' => 'R4',
        'resources' => include('Patient', 'Medication', 'MedicationRequest', 'MedicationStatement',
                               'MedicationAdministration')
      )
    )
    expect(data.dig('client_tools', 'cli')).to include(
      'supported' => true,
      'status' => 'available',
      'binary' => 'medtracker',
      'api_boundary' => '/api/v1',
      'distribution' => 'github_release'
    )
    expect(data.dig('client_tools', 'mcp_server')).to include(
      'supported' => true,
      'transport' => 'streamable_http',
      'endpoint' => '/mcp',
      'stdio_binary' => 'medtracker-mcp',
      'tools' => include(
        'medtracker_current_user',
        'medtracker_household_snapshot',
        'medtracker_today_schedule',
        'medtracker_inventory_risks',
        'medtracker_health_history_summary'
      ),
      'resources' => include('medtracker://household/snapshot')
    )
  end
end
