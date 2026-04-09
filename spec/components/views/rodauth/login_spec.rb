# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Views::Rodauth::Login, type: :component do
  let(:credential_options) do
    Struct.new(:challenge) do
      def as_json
        { challenge: challenge, allowCredentials: [] }
      end
    end.new('challenge')
  end

  # rubocop:disable RSpec/VerifiedDoubles
  let(:rodauth) do
    double(
      'Rodauth',
      login_path: '/login',
      reset_password_request_path: '/reset-password-request',
      create_account_path: '/create-account',
      verify_account_resend_path: '/verify-account-resend',
      webauthn_login_path: '/webauthn-login',
      webauthn_auth_param: 'webauthn_auth',
      webauthn_auth_challenge_param: 'webauthn_auth_challenge',
      webauthn_auth_challenge_hmac_param: 'webauthn_auth_challenge_hmac',
      webauthn_auth_additional_form_tags: '',
      compute_hmac: 'challenge-hmac',
      webauthn_credential_options_for_get: credential_options
    )
  end

  before do
    allow(controller).to receive_messages(rodauth: rodauth, form_authenticity_token: 'token')
    allow(rodauth).to receive(:respond_to?).with(:omniauth_request_path).and_return(false)
    allow(rodauth).to receive(:field_error).and_return(nil)
  end

  it 'renders the login form' do
    rendered = render_inline(described_class.new)

    expect(rendered.text).to include('Welcome back')
    expect(rendered.text).to include('MedTracker')
  end

  it 'renders passkey controls for login autofill and explicit sign-in' do
    rendered = render_inline(described_class.new)

    expect(rendered.text).to include('Continue with Passkey')
    expect(rendered.css('#webauthn-login-form').count).to eq(1)
    expect(rendered.css('#passkey-login-trigger').count).to eq(1)
  end

  it 'renders passkey login form fields and login autocomplete hints' do
    rendered = render_inline(described_class.new)

    expect(rendered.css('input[autocomplete="username webauthn"]').count).to eq(1)
    expect(rendered.to_html).to include('/webauthn-login')
    expect(rendered.to_html).to include('webauthn_auth_challenge')
    expect(rendered.to_html).to include('challenge-hmac')
  end

  it 'renders login form even when passkey credential generation fails' do
    allow(rodauth).to receive(:webauthn_credential_options_for_get).and_raise(StandardError, 'WebAuthn unavailable')

    rendered = render_inline(described_class.new)

    expect(rendered.text).to include('Welcome back')
    expect(rendered.css('#webauthn-login-form').count).to eq(0)
  end

  it 'renders flash message inline near the login form (proximity principle)' do
    flash_hash = ActionDispatch::Flash::FlashHash.new(alert: 'Please login to continue')
    allow(controller).to receive(:flash).and_return(flash_hash)

    rendered = render_inline(described_class.new)

    alert_elements = rendered.css('[role="alert"]')
    expect(alert_elements.length).to eq(1)
    expect(rendered.text).to include('Please login to continue')
  end

  it 'renders Rodauth field errors inline next to form fields, not in flash' do
    allow(rodauth).to receive(:field_error).with('login').and_return('There was an error logging in')

    rendered = render_inline(described_class.new)

    expect(rendered.text).to include('There was an error logging in')
    expect(rendered.css('#login-flash [role="alert"]').count).to eq(0)
  end
  # rubocop:enable RSpec/VerifiedDoubles
end
