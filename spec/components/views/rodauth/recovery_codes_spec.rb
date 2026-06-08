# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Views::Rodauth::RecoveryCodes, type: :component do
  it 'renders the styled password confirmation form', :aggregate_failures do
    rodauth = recovery_codes_auth(can_add: true, codes: [])

    allow(controller).to receive_messages(rodauth: rodauth, form_authenticity_token: 'token')

    rendered = render_inline(described_class.new)

    expect(rendered.text).to include('Recovery codes')
    expect(rendered.text).to include('Confirm your password to view or generate backup codes.')
    expect(rendered.to_html).to include('action="/recovery-codes"')
    expect(rendered.to_html).to include('name="password"')
    expect(rendered.to_html).to include('View Authentication Recovery Codes')
    expect(rendered.to_html).to include('min-h-screen')
  end

  def recovery_codes_auth(can_add:, codes:, button: 'View Authentication Recovery Codes')
    RodauthApp.rodauth.allocate.tap do |rodauth|
      allow(rodauth).to receive_messages(
        recovery_codes_path: '/recovery-codes',
        recovery_codes: codes,
        recovery_codes_button: button,
        view_recovery_codes_button: 'View Authentication Recovery Codes',
        recovery_codes_additional_form_tags: '',
        add_recovery_codes_param: 'add',
        can_add_recovery_codes?: can_add,
        two_factor_modifications_require_password?: true,
        password_param: 'password',
        password_label: 'Password',
        field_error: nil
      )
    end
  end
end
