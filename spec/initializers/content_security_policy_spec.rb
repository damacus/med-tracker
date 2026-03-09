# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Content Security Policy Configuration' do # rubocop:disable RSpec/DescribeClass
  let(:csp) { Rails.application.config.content_security_policy }

  describe 'nonce directives' do
    it 'applies nonces to script-src only' do
      expect(Rails.application.config.content_security_policy_nonce_directives).to eq(%w[script-src])
    end

    it 'does not apply nonces to style-src (Turbo injects inline styles without nonce)' do
      expect(Rails.application.config.content_security_policy_nonce_directives).not_to include('style-src')
    end
  end

  describe 'policy directives' do
    it 'configures style-src and script-src to self' do
      expect(csp.directives['style-src']).to eq(["'self'"])
      expect(csp.directives['script-src']).to eq(["'self'"])
    end

    it 'does not allow unsafe-inline in style-src' do
      expect(csp.directives['style-src']).not_to include("'unsafe-inline'")
    end
  end
end
