# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'SMART on FHIR discovery' do
  it 'publishes the supported authorization contract without authentication' do
    get '/api/fhir/R4/.well-known/smart-configuration'

    expect(response).to have_http_status(:ok)
    expect(response.media_type).to eq('application/json')
    expect(response.parsed_body).to include(
      'authorization_endpoint' => end_with('/authorize'),
      'token_endpoint' => end_with('/token'),
      'revocation_endpoint' => end_with('/revoke'),
      'code_challenge_methods_supported' => ['S256'],
      'capabilities' => include('launch-standalone', 'client-public')
    )
  end
end
