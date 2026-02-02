# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Views::Rodauth::WebauthnAuth, type: :component do
  # rubocop:disable RSpec/ExampleLength, RSpec/VerifiedDoubles
  it 'renders the WebAuthn authentication form' do
    credential_options = Struct.new(:challenge) do
      def as_json
        { challenge: challenge }
      end
    end.new('challenge')

    rodauth = double(
      'Rodauth',
      webauthn_auth_param: 'webauthn-auth',
      webauthn_auth_form_path: '/webauthn-auth',
      webauthn_auth_challenge_param: 'webauthn_challenge',
      webauthn_auth_challenge_hmac_param: 'webauthn_challenge_hmac',
      webauthn_auth_additional_form_tags: '',
      webauthn_auth_button: 'Use passkey',
      webauthn_js_host: '',
      webauthn_auth_js_path: '/webauthn-auth.js',
      compute_hmac: 'hmac',
      webauthn_credential_options_for_get: credential_options
    )

    allow(controller).to receive_messages(rodauth: rodauth, form_authenticity_token: 'token')

    rendered = render_inline(described_class.new)

    expect(rendered.text).to include('Use your passkey')
    expect(rendered.to_html).to include('webauthn-auth')
    expect(rendered.to_html).to include('Use passkey')
    expect(rendered.to_html).to include('min-h-screen')
  end
  # rubocop:enable RSpec/ExampleLength, RSpec/VerifiedDoubles
end
