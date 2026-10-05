# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Views::Rodauth::OtpAuth, type: :component do
  # rubocop:disable RSpec/VerifiedDoubles, RSpec/ExampleLength
  it 'renders the OTP authentication form' do
    rodauth = double(
      'Rodauth',
      otp_auth_param: 'otp-auth',
      otp_auth_path: '/otp-auth',
      otp_auth_button: 'Verify code',
      otp_auth_label: 'Authentication code',
      otp_auth_additional_form_tags: '',
      field_error: nil
    )

    controller.request.env['rodauth'] = rodauth
    allow(view_context).to receive_messages(rodauth: rodauth, form_authenticity_token: 'token', flash: {})

    rendered = render_inline(described_class.new)

    expect(rendered.text).to include('Enter your authentication code')
    expect(rendered.to_html).to include('otp-auth')
    expect(rendered.to_html).to include('Verify code')
    expect(rendered.to_html).to include('min-h-screen')
  end

  it 'renders the challenge as a six-slot OTP input' do
    rodauth = double(
      'Rodauth',
      otp_auth_param: 'otp-auth',
      otp_auth_path: '/otp-auth',
      otp_auth_button: 'Verify code',
      otp_auth_label: 'Authentication code',
      otp_auth_additional_form_tags: '',
      field_error: nil
    )
    controller.request.env['rodauth'] = rodauth
    allow(view_context).to receive_messages(rodauth: rodauth, form_authenticity_token: 'token', flash: {})

    rendered = render_inline(described_class.new)
    input = rendered.at_css('input#otp-auth-code[name="otp-auth"]')

    expect(input['aria-label']).to eq('Authentication code')
    expect(input['autocomplete']).to eq('one-time-code')
    expect(rendered.css('[data-ruby-ui--input-otp-target="slot"]').count).to eq(6)
  end
  # rubocop:enable RSpec/VerifiedDoubles, RSpec/ExampleLength
end
