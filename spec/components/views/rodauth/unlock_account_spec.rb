# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Views::Rodauth::UnlockAccount, type: :component do
  let(:rodauth) do
    double(
      'Rodauth',
      unlock_account_path: '/unlock-account',
      unlock_account_button: 'Unlock Account',
      unlock_account_explanatory_text: '<p>You can unlock the account.</p>',
      unlock_account_additional_form_tags: '',
      unlock_account_requires_password?: false,
      unlock_account_key_param: 'key'
    )
  end

  before do
    allow(controller).to receive_messages(
      rodauth: rodauth,
      form_authenticity_token: 'token',
      flash: {},
      params: { 'key' => 'unlock-key' }
    )
  end

  it 'renders the unlock account form in the auth shell' do
    rendered = render_inline(described_class.new)

    expect(rendered.text).to include('Unlock Account')
    expect(rendered.text).to include('You can unlock the account.')
    expect(rendered.css('form[action="/unlock-account"]').count).to eq(1)
    expect(rendered.css('input[name="key"][value="unlock-key"]').count).to eq(1)
  end
end
