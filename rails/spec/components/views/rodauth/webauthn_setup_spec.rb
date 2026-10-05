# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Views::Rodauth::WebauthnSetup, type: :component do
  let(:credential_options) do
    Struct.new(:challenge) do
      def as_json
        { challenge: challenge, rp: { name: 'MedTracker' } }
      end
    end.new('test-challenge')
  end

  # rubocop:disable RSpec/VerifiedDoubles
  let(:rodauth) do
    double(
      'Rodauth',
      webauthn_setup_path: '/webauthn-setup',
      webauthn_setup_param: 'webauthn_setup',
      webauthn_setup_challenge_param: 'webauthn_setup_challenge',
      webauthn_setup_challenge_hmac_param: 'webauthn_setup_challenge_hmac',
      webauthn_setup_additional_form_tags: '',
      webauthn_js_host: '',
      webauthn_setup_js_path: '/webauthn-setup.js',
      new_webauthn_credential: credential_options,
      compute_hmac: 'hmac-value',
      current_route: :webauthn_setup
    )
  end

  before do
    allow(controller).to receive_messages(rodauth: rodauth, form_authenticity_token: 'token',
                                          content_security_policy_nonce: 'test-nonce')
  end

  it 'renders the webauthn setup form' do
    rendered = render_inline(described_class.new)

    expect(rendered.text).to include('Register a Passkey')
    expect(rendered.to_html).to include('webauthn-setup-form')
  end

  it 'renders graceful error when credential generation fails' do
    allow(rodauth).to receive(:new_webauthn_credential).and_raise(StandardError, 'WebAuthn error')

    rendered = render_inline(described_class.new)

    expect(rendered.text).to include('Unable to initialize passkey registration')
    expect(rendered.to_html).not_to include('webauthn-setup-form')
  end
  # rubocop:enable RSpec/VerifiedDoubles
end
