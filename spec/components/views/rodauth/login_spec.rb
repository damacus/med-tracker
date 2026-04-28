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

  it 'renders the split auth surface from the storyboard reference' do
    rendered = render_inline(described_class.new)

    landmark_counts = {
      surface: rendered.css('[data-login-surface="split-auth"]').count,
      brand_panel: rendered.css('[data-login-panel="brand"]').count,
      form_panel: rendered.css('[data-login-panel="form"]').count,
      illustration: rendered.css('[data-login-illustration="medication"]').count
    }
    expect(landmark_counts).to eq(surface: 1, brand_panel: 1, form_panel: 1, illustration: 1)
    expect(rendered.css('svg[data-login-logo="mt"]').count).to eq(1)
    illustration = rendered.at_css('[data-login-illustration="medication"]')
    expect(illustration['class']).not_to include('hidden')
    expect(illustration.css('[data-login-illustration-layer]').count).to eq(0)
    expect(illustration.css('canvas').count).to eq(0)
    expect(illustration.css('[style*="background-image"]').count).to eq(0)
    expect(illustration.css('picture.login-med-illustration__picture--light').count).to eq(1)
    expect(illustration.css('picture.login-med-illustration__picture--dark').count).to eq(1)
    %w[
      login-med-illustration-light-desktop
      login-med-illustration-dark-mobile
      login-med-illustration-light-mobile
      login-med-illustration-dark-desktop
    ].each do |asset|
      expect(illustration.to_html).to include(asset)
    end
    expect(rendered.css('svg[data-login-benefit-icon="stay-on-track"]').count).to eq(1)
    benefit_list = rendered.at_css('[data-login-benefits]')
    expect(benefit_list['class']).to include('hidden')
    expect(benefit_list['class']).to include('md:block')
    expect(rendered.css('svg[data-login-benefit-icon="schedule"]').count).to eq(1)
    expect(rendered.to_html).to include('M200-80q-33 0-56.5-23.5T120-160')
    expect(rendered.css('svg[data-login-benefit-icon="progress"]').count).to eq(1)
    expect(rendered.css('svg[data-login-benefit-icon="insights"]').count).to eq(1)
    expect(rendered.to_html).to include('M400-320q100 0 170-70t70-170')
    expect(rendered.text).to include('Stay on track', 'Your schedule', 'Track your progress',
                                     'Insights that help', 'Other sign-in options')
  end

  it 'places the welcome copy in the brand panel instead of the form panel' do
    rendered = render_inline(described_class.new)

    brand_panel = rendered.at_css('[data-login-panel="brand"]')
    form_panel = rendered.at_css('[data-login-panel="form"]')

    expect(brand_panel.css('h1').text).to include('Welcome back')
    expect(brand_panel.text).to include('Sign in to your account to continue')
    expect(form_panel.css('h1')).to be_empty
  end

  it 'renders passkey controls for login autofill and explicit sign-in' do
    rendered = render_inline(described_class.new)

    expect(rendered.text).to include('Continue with Passkey')
    expect(rendered.css('#webauthn-login-form').count).to eq(1)
    expect(rendered.css('#passkey-login-trigger').count).to eq(1)
    expect(rendered.css('svg[data-login-sign-in-icon="passkey"]').count).to eq(1)
    expect(rendered.to_html).to include('M3 20v-2.35c0 -0.63335')
    expect(rendered.css('svg[data-login-sign-in-chevron="passkey"] path[d="M9 5L16 12L9 19"]').count).to eq(1)
  end

  it 'renders the OIDC sign-in option with the supplied SSO icon when available' do
    allow(rodauth).to receive(:respond_to?).with(:omniauth_request_path).and_return(true)
    allow(rodauth).to receive(:omniauth_request_path).with(:oidc).and_return('/auth/oidc')
    allow(Rails.application.credentials).to receive(:dig).and_call_original
    allow(Rails.application.credentials).to receive(:dig).with(:oidc, :client_id).and_return('test-client-id')
    allow(Rails.application.credentials).to receive(:dig).with(:oidc, :issuer_url).and_return('https://issuer.example.com')
    allow(User).to receive(:administrator).and_return(instance_double(ActiveRecord::Relation, exists?: false))

    rendered = render_inline(described_class.new)

    expect(rendered.text).to include('Login with OIDC (SSO)')
    expect(rendered.css('svg[data-login-sign-in-icon="sso"]').count).to eq(1)
    expect(rendered.to_html).to include('M480-380Zm80 220H260q-91 0-155.5-63T40-377')
    expect(rendered.css('svg[data-login-sign-in-chevron="sso"] path[d="M9 5L16 12L9 19"]').count).to eq(1)
  end

  it 'hides secondary sign-in options until a visible option is available' do
    rendered = render_inline(described_class.new)

    expect(rendered.css('#secondary-sign-in-options[hidden]').count).to eq(1)
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
    expect(rendered.css('#login-flash [role="alert"]').count).to eq(1)
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
