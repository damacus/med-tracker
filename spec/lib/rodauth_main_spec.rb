# frozen_string_literal: true

require 'rails_helper'

RSpec.describe RodauthMain do
  describe '#set_redirect_error_flash' do
    it 'does not set flash for the routine login-required redirect' do
      auth = RodauthApp.rodauth.allocate
      flash_hash = ActionDispatch::Flash::FlashHash.new
      allow(auth).to receive(:flash).and_return(flash_hash)

      auth.send(:set_redirect_error_flash, auth.send(:require_login_error_flash))

      expect(flash_hash).to be_empty
    end

    it 'keeps non-routine redirect errors visible' do
      auth = RodauthApp.rodauth.allocate
      flash_hash = ActionDispatch::Flash::FlashHash.new
      allow(auth).to receive(:flash).and_return(flash_hash)

      auth.send(:set_redirect_error_flash, 'Your session expired')

      expect(flash_hash[:alert]).to eq('Your session expired')
    end
  end

  describe '#before_omniauth_create_account' do
    it 'redirects to login with the invite-only notice when registrations are closed' do
      auth = RodauthApp.rodauth.allocate
      message = I18n.t('sessions.login.invite_only_oidc_notice',
                       default: 'Single sign-on is reserved for invited accounts.')
      allow(auth).to receive(:invite_only_registration_required?).and_return(true)
      allow(auth).to receive(:set_redirect_error_flash)
      allow(auth).to receive(:redirect)

      auth.send(:before_omniauth_create_account)

      expect(auth).to have_received(:set_redirect_error_flash).with(message)
      expect(auth).to have_received(:redirect).with(auth.login_path)
    end
  end

  describe '#two_factor_auth_return_to_requested_location?' do
    it 'returns users to the protected action after additional authentication' do
      auth = RodauthApp.rodauth.allocate

      expect(auth.two_factor_auth_return_to_requested_location?).to be true
    end
  end
end
