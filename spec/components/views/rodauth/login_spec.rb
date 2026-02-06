# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Views::Rodauth::Login, type: :component do
  # rubocop:disable RSpec/VerifiedDoubles
  let(:rodauth) do
    double(
      'Rodauth',
      login_path: '/login',
      reset_password_request_path: '/reset-password-request',
      create_account_path: '/create-account',
      verify_account_resend_path: '/verify-account-resend'
    )
  end

  before do
    allow(controller).to receive_messages(rodauth: rodauth, form_authenticity_token: 'token')
    allow(rodauth).to receive(:respond_to?).with(:omniauth_request_path).and_return(false)
  end

  it 'renders the login form' do
    rendered = render_inline(described_class.new)

    expect(rendered.text).to include('Welcome back')
    expect(rendered.text).to include('MedTracker')
  end

  it 'renders flash message inline near the login form (proximity principle)' do
    flash_hash = ActionDispatch::Flash::FlashHash.new(alert: 'Please login to continue')
    allow(controller).to receive(:flash).and_return(flash_hash)

    rendered = render_inline(described_class.new)

    alert_elements = rendered.css('[role="alert"]')
    expect(alert_elements.length).to eq(1)
    expect(rendered.text).to include('Please login to continue')
  end
  # rubocop:enable RSpec/VerifiedDoubles
end
