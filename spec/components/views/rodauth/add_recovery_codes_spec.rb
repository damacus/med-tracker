# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Views::Rodauth::AddRecoveryCodes, type: :component do
  it 'renders generated codes in the styled recovery code grid', :aggregate_failures do
    rodauth = recovery_codes_auth(
      can_add: true,
      codes: %w[alpha-code bravo-code],
      button: 'Add Authentication Recovery Codes'
    )

    allow(controller).to receive_messages(rodauth: rodauth, form_authenticity_token: 'token')

    rendered = render_inline(described_class.new)

    expect(rendered.to_html).to include('id="recovery-codes"')
    expect(rendered.text).to include('alpha-code')
    expect(rendered.text).to include('bravo-code')
    expect(rendered.to_html).to include('name="add"')
    expect(rendered.text).to include('Store these codes somewhere safe.')
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
