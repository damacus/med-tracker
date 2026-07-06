# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'API v1 capabilities' do
  it 'publishes the supported API, auth, portability, sync, and deferred client-tool contracts without auth' do
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
    expect(data.dig('sync', 'portable_ids')).to be(true)
    expect(data.dig('sync', 'numeric_ids')).to eq('backward_compatible')
    expect(data.dig('client_tools', 'cli')).to include('supported' => false, 'status' => 'deferred')
    expect(data.dig('client_tools', 'mcp_server')).to include('supported' => false, 'status' => 'deferred')
  end
end
