# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Views::Rodauth::RecoveryAuth, type: :component do
  it 'renders the recovery code authentication form', :aggregate_failures do
    rodauth = recovery_auth
    controller.request.env['rodauth'] = rodauth
    allow(view_context).to receive_messages(rodauth: rodauth, form_authenticity_token: 'token', flash: {})

    rendered = render_inline(described_class.new)

    expect(rendered.text).to include('Additional authentication required')
    expect(rendered.text).to include('Recovery Code')
    expect(rendered.text).to include('Use one of your saved recovery codes.')
    expect(rendered.to_html).to include('/recovery-auth')
    expect(rendered.to_html).to include('recovery-code')
    expect(rendered.to_html).to include('min-h-screen')
  end

  def recovery_auth
    RodauthApp.rodauth.allocate.tap do |rodauth|
      allow(rodauth).to receive_messages(
        recovery_auth_path: '/recovery-auth',
        recovery_auth_page_title: 'Recovery Code',
        recovery_auth_button: 'Verify code',
        recovery_auth_additional_form_tags: '',
        recovery_codes_param: 'recovery-code',
        recovery_codes_label: 'Recovery Code',
        field_error: nil
      )
    end
  end
end
