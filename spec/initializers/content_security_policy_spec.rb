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
    it 'sets style-src to self only' do
      policy = ActionDispatch::ContentSecurityPolicy.new
      csp.call(policy)
      header = policy.build(nil)

      expect(header).to include("style-src 'self'")
      expect(header).not_to match(/style-src.*unsafe-inline/)
    end

    it 'sets script-src to self only' do
      policy = ActionDispatch::ContentSecurityPolicy.new
      csp.call(policy)
      header = policy.build(nil)

      expect(header).to include("script-src 'self'")
    end
  end
end
