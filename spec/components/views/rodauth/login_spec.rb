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

  it 'uses focused auth visual components for login artwork' do
    expect(Components::Auth::MtLogo).to be < Components::Base
    expect(Components::Auth::BenefitIconTile).to be < Components::Base
    expect(Components::Auth::MedicationIllustration).to be < Components::Base
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
  end

  it 'renders the new bar and check MedTracker logo' do
    rendered = render_inline(described_class.new)
    logo = rendered.at_css('svg[data-login-logo="mt"]')

    expect(logo['viewbox']).to eq('0 0 192 96')
    expect(logo.css('[data-login-logo-part="bar"]').count).to eq(3)
    expect(logo.css('[data-login-logo-part="check-badge"]').count).to eq(1)
    expect(logo.css('[data-login-logo-part="check-mark"]').count).to eq(1)
  end

  it 'renders the medication illustration without generated layers' do
    rendered = render_inline(described_class.new)
    illustration = rendered.at_css('[data-login-illustration="medication"]')
    illustration_counts = {
      layers: illustration.css('[data-login-illustration-layer]').count,
      canvases: illustration.css('canvas').count,
      backgrounds: illustration.css('[style*="background-image"]').count
    }

    expect(illustration['class']).not_to include('hidden')
    expect(illustration_counts).to eq(layers: 0, canvases: 0, backgrounds: 0)
  end

  it 'renders one medication illustration picture with deferred theme assets' do
    rendered = render_inline(described_class.new)
    illustration = rendered.at_css('[data-login-illustration="medication"]')
    picture_counts = {
      pictures: illustration.css('picture.login-med-illustration__picture').count,
      images: illustration.css('img.login-med-illustration__image').count
    }

    expect(picture_counts).to eq(pictures: 1, images: 1)
    expect(illustration.css('img.login-med-illustration__image[src]')).to be_empty
    expect(illustration.css('picture.login-med-illustration__picture--dark')).to be_empty
    expect(illustration.to_html).to include(
      'login-med-illustration-light-desktop',
      'login-med-illustration-dark-mobile',
      'login-med-illustration-light-mobile',
      'login-med-illustration-dark-desktop'
    )
  end

  it 'renders the login benefits from the storyboard reference' do
    rendered = render_inline(described_class.new)
    benefit_list = rendered.at_css('[data-login-benefits]')
    benefit_counts = {
      stay_on_track: rendered.css('svg[data-login-benefit-icon="stay-on-track"]').count,
      schedule: rendered.css('svg[data-login-benefit-icon="schedule"]').count,
      progress: rendered.css('svg[data-login-benefit-icon="progress"]').count,
      insights: rendered.css('svg[data-login-benefit-icon="insights"]').count
    }

    expect(benefit_counts).to eq(stay_on_track: 1, schedule: 1, progress: 1, insights: 1)
    expect(benefit_list['class']).to include('hidden', 'md:block')
    expect(rendered.to_html).to include('M200-80q-33 0-56.5-23.5T120-160', 'M400-320q100 0 170-70t70-170')
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
    passkey_control_counts = {
      form: rendered.css('#webauthn-login-form').count,
      trigger: rendered.css('#passkey-login-trigger').count,
      icon: rendered.css('svg[data-login-sign-in-icon="passkey"]').count,
      chevron: rendered.css('svg[data-login-sign-in-chevron="passkey"] path[d="M9 5L16 12L9 19"]').count
    }

    expect(rendered.text).to include('Continue with Passkey')
    expect(passkey_control_counts).to eq(form: 1, trigger: 1, icon: 1, chevron: 1)
    expect(rendered.to_html).to include('M3 20v-2.35c0 -0.63335')
  end

  it 'renders the OIDC sign-in option with the supplied SSO icon when available' do
    allow(rodauth).to receive(:respond_to?).with(:omniauth_request_path).and_return(true)
    allow(rodauth).to receive(:omniauth_request_path).with(:oidc).and_return('/auth/oidc')
    allow(Rails.application.credentials).to receive(:dig).and_call_original
    allow(Rails.application.credentials).to receive(:dig).with(:oidc, :client_id).and_return('test-client-id')
    allow(Rails.application.credentials).to receive(:dig).with(:oidc, :issuer_url).and_return('https://issuer.example.com')

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

  it 'suppresses routine login-required flash near the login form' do
    flash_hash = ActionDispatch::Flash::FlashHash.new(alert: 'Please login to continue')
    allow(controller).to receive(:flash).and_return(flash_hash)

    rendered = render_inline(described_class.new)

    expect(rendered.css('#login-flash [role="alert"]').count).to eq(0)
    expect(rendered.text).not_to include('Please login to continue')
  end

  it 'renders non-routine flash message inline near the login form' do
    flash_hash = ActionDispatch::Flash::FlashHash.new(alert: 'Your session expired')
    allow(controller).to receive(:flash).and_return(flash_hash)

    rendered = render_inline(described_class.new)

    expect(rendered.css('[role="alert"]').length).to eq(1)
    expect(rendered.css('#login-flash [role="alert"]').count).to eq(1)
    expect(rendered.text).to include('Your session expired')
  end

  it 'renders Rodauth field errors inline next to form fields, not in flash' do
    allow(rodauth).to receive(:field_error).with('login').and_return('There was an error logging in')

    rendered = render_inline(described_class.new)

    expect(rendered.text).to include('There was an error logging in')
    expect(rendered.css('#login-flash [role="alert"]').count).to eq(0)
  end
  # rubocop:enable RSpec/VerifiedDoubles
end
