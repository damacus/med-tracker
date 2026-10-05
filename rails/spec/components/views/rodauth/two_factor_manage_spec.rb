# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Views::Rodauth::TwoFactorManage, type: :component do
  it 'renders setup actions when two-factor authentication is not configured' do
    rodauth = manage_rodauth(two_factor_setup: false)
    allow(controller).to receive(:rodauth).and_return(rodauth)

    rendered = render_inline(described_class.new)

    expect(rendered.text).to include('Set Up Two-Factor Authentication')
    expect(rendered.to_html).to include('/webauthn-setup')
    expect(rendered.to_html).to include('/otp-setup')
    expect(rendered.to_html).not_to include('/recovery-codes')
  end

  it 'renders management actions when two-factor authentication is configured' do
    rodauth = manage_rodauth(
      two_factor_setup: true,
      webauthn_enabled: true,
      totp_enabled: true,
      recovery_codes_enabled: true
    )
    allow(controller).to receive(:rodauth).and_return(rodauth)

    rendered = render_inline(described_class.new)

    expect(rendered.text).to include('Manage Two-Factor Authentication')
    expect(rendered.text).to include('Recovery Codes')
    expect(rendered.to_html).to include('/webauthn-remove')
    expect(rendered.to_html).to include('/otp-disable')
  end

  it 'treats two-factor method lookup errors as disabled methods' do
    rodauth = manage_rodauth(two_factor_setup: true)
    rodauth.define_singleton_method(:uses_webauthn_authentication?) { raise StandardError }
    rodauth.define_singleton_method(:otp_exists?) { raise StandardError }
    rodauth.define_singleton_method(:recovery_codes_exist?) { raise StandardError }
    allow(controller).to receive(:rodauth).and_return(rodauth)

    rendered = render_inline(described_class.new)

    expect(rendered.to_html).to include('/webauthn-setup')
    expect(rendered.to_html).to include('/otp-setup')
    expect(rendered.to_html).to include('/recovery-codes')
  end

  def manage_rodauth(two_factor_setup:, webauthn_enabled: false, totp_enabled: false, recovery_codes_enabled: false)
    Object.new.tap do |rodauth|
      {
        two_factor_authentication_setup?: two_factor_setup,
        uses_webauthn_authentication?: webauthn_enabled,
        otp_exists?: totp_enabled,
        recovery_codes_exist?: recovery_codes_enabled,
        webauthn_setup_path: '/webauthn-setup',
        webauthn_remove_path: '/webauthn-remove',
        otp_setup_path: '/otp-setup',
        otp_disable_path: '/otp-disable',
        recovery_codes_path: '/recovery-codes'
      }.each do |method_name, value|
        rodauth.define_singleton_method(method_name) { value }
      end
    end
  end
end
