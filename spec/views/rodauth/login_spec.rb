# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Views::Rodauth::Login do
  fixtures :accounts, :people, :users

  def credential_options
    Struct.new(:challenge) do
      def as_json
        { challenge: challenge, allowCredentials: [] }
      end
    end.new('challenge')
  end

  def render_login(oauth_enabled: false, invite_only: nil)
    rodauth_mock = setup_rodauth_mock(oauth_enabled: oauth_enabled)
    vc = view_context
    controller.request.env['rodauth'] = rodauth_mock

    allow(vc).to receive_messages(rodauth: rodauth_mock, flash: {}, params: {})
    stub_invite_only(invite_only)

    render_inline(described_class.new)
  end

  private

  def setup_rodauth_mock(oauth_enabled: false)
    mock = double('Rodauth') # rubocop:disable RSpec/VerifiedDoubles
    allow(mock).to receive_messages(**rodauth_mock_messages)
    allow(mock).to receive(:respond_to?).with(:omniauth_request_path).and_return(oauth_enabled)
    if oauth_enabled
      allow(mock).to receive(:omniauth_request_path).with(:oidc).and_return('/auth/oidc')
      stub_oidc_credentials
    end
    mock
  end

  def rodauth_mock_messages
    {
      login_path: '/login',
      reset_password_request_path: '/reset-password',
      create_account_path: '/create-account',
      verify_account_resend_path: '/resend-verify',
      webauthn_login_path: '/webauthn-login',
      webauthn_auth_param: 'webauthn_auth',
      webauthn_auth_challenge_param: 'webauthn_auth_challenge',
      webauthn_auth_challenge_hmac_param: 'webauthn_auth_challenge_hmac',
      webauthn_auth_additional_form_tags: '',
      compute_hmac: 'challenge-hmac',
      webauthn_credential_options_for_get: credential_options
    }
  end

  def stub_oidc_credentials
    creds = Rails.application.credentials
    allow(creds).to receive(:dig).and_call_original
    allow(creds).to receive(:dig).with(:oidc, :client_id).and_return('test-client-id')
    allow(creds).to receive(:dig).with(:oidc, :issuer_url).and_return('https://issuer.example.com')
  end

  def stub_invite_only(invite_only)
    return if invite_only.nil?

    allow(User).to receive(:administrator).and_return(instance_double(ActiveRecord::Relation, exists?: invite_only))
  end

  it 'renders the login heading and brand' do
    rendered = render_login
    expect(rendered.text).to include('MedTracker')
    expect(rendered.text).to include('Welcome back')
  end

  it 'renders the email and password fields' do
    rendered = render_login
    expect(rendered.text).to include('Email address')
    expect(rendered.text).to include('Password')
  end

  it 'renders the remember me checkbox' do
    rendered = render_login
    expect(rendered.text).to include('Remember me')
  end

  it 'renders the sign in button' do
    rendered = render_login
    expect(rendered.text).to include('Sign In to Dashboard')
  end

  it 'renders the hidden passkey login form and CTA' do
    rendered = render_login

    expect(rendered.text).to include('Continue with Passkey')
    expect(rendered.css('#webauthn-login-form').count).to eq(1)
    expect(rendered.css('#passkey-login-trigger').count).to eq(1)
    expect(rendered.css('input[autocomplete="username webauthn"]').count).to eq(1)
  end

  it 'renders the OAuth button when invite-only is disabled' do
    rendered = render_login(oauth_enabled: true, invite_only: false)

    expect(rendered.text).to include('or continue with')
    expect(rendered.text).to include('OIDC')
  end

  it 'hides the OAuth CTA and shows invite-only notice when invite-only is active' do
    rendered = render_login(oauth_enabled: true, invite_only: true)

    expect(rendered.text).not_to include('or continue with')
    expect(rendered.text).to include('Single sign-on is reserved for invited accounts.')
  end
end
