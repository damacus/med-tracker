# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Views::Rodauth::OtpSetup, type: :component do
  let(:rodauth) do
    RodauthApp.rodauth.allocate.tap do |instance|
      allow(instance).to receive_messages(
        otp_qr_code: '<svg></svg>',
        otp_user_key: 'secret-key',
        otp_provisioning_uri: 'otpauth://totp/MedTracker',
        otp_setup_path: '/otp-setup',
        otp_setup_param: 'otp-secret',
        otp_setup_raw_param: 'otp-raw',
        otp_key: 'raw-secret',
        otp_keys_use_hmac?: false,
        otp_auth_param: 'otp'
      )
    end
  end

  before do
    allow(controller).to receive_messages(rodauth: rodauth, form_authenticity_token: 'token')
  end

  it 'gives the copy button an accessible name and hides its icon' do
    rendered = render_inline(described_class.new)
    copy_button = rendered.at_css('button[aria-label="Copy to clipboard"]')

    expect(copy_button).to be_present
    expect(copy_button.at_css('svg[aria-hidden="true"]')).to be_present
  end
end
