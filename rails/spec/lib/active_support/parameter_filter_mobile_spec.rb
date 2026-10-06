# frozen_string_literal: true

require 'rails_helper'

RSpec.describe ActiveSupport::ParameterFilter do
  it 'filters authorization codes, PKCE verifiers and tokens from request diagnostics' do
    filter = described_class.new(Rails.application.config.filter_parameters)
    secrets = %w[code code_verifier code_challenge access_token refresh_token id_token]
    params = secrets.index_with { |key| "private-#{key}" }.merge('client_id' => 'public-client')

    expect(filter.filter(params)).to eq(secrets.index_with { '[FILTERED]' }.merge('client_id' => 'public-client'))
  end
end
