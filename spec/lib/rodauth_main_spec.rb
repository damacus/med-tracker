# frozen_string_literal: true

require 'rails_helper'

RSpec.describe RodauthMain do
  describe '#before_omniauth_create_account' do
    it 'redirects to login with the invite-only notice when registrations are closed' do
      auth = RodauthApp.rodauth.allocate
      allow(auth).to receive(:invite_only_registration_required?).and_return(true)
      allow(auth).to receive(:set_notice_flash)
      allow(auth).to receive(:redirect)

      auth.send(:before_omniauth_create_account)

      expect(auth).to have_received(:set_notice_flash).with(auth.send(:invite_only_registration_message))
      expect(auth).to have_received(:redirect).with(auth.login_path)
    end
  end
end
