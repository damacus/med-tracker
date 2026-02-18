# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Views::Rodauth::Login do
  fixtures :accounts, :people, :users

  def render_login
    rodauth_mock = setup_rodauth_mock
    vc = view_context
    # Satisfy rodauth-rails internal environment check
    controller.request.env['rodauth'] = rodauth_mock

    allow(vc).to receive_messages(rodauth: rodauth_mock, flash: {}, params: {})

    render_inline(described_class.new)
  end

  private

  def setup_rodauth_mock
    mock = double('Rodauth') # rubocop:disable RSpec/VerifiedDoubles
    allow(mock).to receive_messages(
      login_path: '/login',
      reset_password_request_path: '/reset-password',
      create_account_path: '/create-account',
      verify_account_resend_path: '/resend-verify'
    )
    allow(mock).to receive(:respond_to?).with(:omniauth_request_path).and_return(false)
    mock
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
end
