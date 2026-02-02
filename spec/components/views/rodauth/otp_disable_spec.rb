# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Views::Rodauth::OtpDisable, type: :component do
  # rubocop:disable RSpec/VerifiedDoubles
  it 'renders the OTP disable form' do
    rodauth = double(
      'Rodauth',
      otp_disable_path: '/otp-disable',
      otp_disable_button: 'Disable TOTP Authentication',
      otp_disable_additional_form_tags: '',
      two_factor_modifications_require_password?: true
    )

    allow(controller).to receive_messages(rodauth: rodauth, form_authenticity_token: 'token')

    rendered = render_inline(described_class.new)

    expect(rendered.text).to include('Disable authenticator app')
    expect(rendered.to_html).to include('action="/otp-disable"')
    expect(rendered.to_html).to include('Disable TOTP Authentication')
    expect(rendered.to_html).to include('min-h-screen')
  end
  # rubocop:enable RSpec/VerifiedDoubles
end
