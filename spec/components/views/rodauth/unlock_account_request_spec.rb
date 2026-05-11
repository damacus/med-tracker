# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Views::Rodauth::UnlockAccountRequest, type: :component do
  # rubocop:disable RSpec/VerifiedDoubles
  let(:rodauth) do
    double(
      'Rodauth',
      unlock_account_request_path: '/unlock-account-request',
      unlock_account_request_button: 'Request Account Unlock',
      unlock_account_request_explanatory_text: '<p>This account is currently locked out.</p>',
      unlock_account_request_additional_form_tags: '',
      login_param: 'email',
      login_path: '/login',
      reset_password_request_path: '/reset-password-request'
    )
  end
  # rubocop:enable RSpec/VerifiedDoubles

  before do
    allow(controller).to receive_messages(rodauth: rodauth, form_authenticity_token: 'token', flash: {}, params: {})
  end

  it 'renders the unlock account request form in the auth shell' do
    rendered = render_inline(described_class.new)

    expect(rendered.text).to include('Request Account Unlock')
    expect(rendered.text).to include('This account is currently locked out.')
    expect(rendered.css('form[action="/unlock-account-request"]').count).to eq(1)
    expect(rendered.css('input[name="email"]').count).to eq(1)
  end
end
