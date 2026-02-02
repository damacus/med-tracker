# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Views::Rodauth::OtpAuth, type: :component do
  # rubocop:disable RSpec/VerifiedDoubles
  it 'renders the OTP authentication form' do
    rodauth = double(
      'Rodauth',
      otp_auth_param: 'otp-auth',
      otp_auth_path: '/otp-auth',
      otp_auth_button: 'Verify code',
      otp_auth_label: 'Authentication code',
      otp_auth_additional_form_tags: ''
    )

    allow(controller).to receive_messages(rodauth: rodauth, form_authenticity_token: 'token')

    rendered = render_inline(described_class.new)

    expect(rendered.text).to include('Enter your authentication code')
    expect(rendered.to_html).to include('otp-auth')
    expect(rendered.to_html).to include('Verify code')
    expect(rendered.to_html).to include('min-h-screen')
  end
  # rubocop:enable RSpec/VerifiedDoubles
end
