# frozen_string_literal: true

require 'rails_helper'

FilterParameterLogging = ActiveSupport::ParameterFilter

RSpec.describe FilterParameterLogging do
  it 'filters OIDC authorization, PKCE, nonce, and token credentials' do
    credentials = {
      authorization_code: 'authorization-code',
      code_verifier: 'pkce-code-verifier',
      nonce: 'oidc-nonce',
      id_token: 'id-token',
      access_token: 'access-token',
      refresh_token: 'refresh-token'
    }
    filtered = described_class.new(Rails.application.config.filter_parameters).filter(credentials)

    expect(filtered.values).to all(eq('[FILTERED]'))
  end
end
