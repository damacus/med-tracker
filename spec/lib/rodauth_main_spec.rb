# frozen_string_literal: true

require 'rails_helper'

RSpec.describe RodauthMain do
  fixtures :people

  describe 'SMART OAuth provider' do
    subject(:auth) { RodauthApp.rodauth.allocate }

    it 'requires PKCE and publishes the supported SMART scopes' do
      expect(auth.oauth_require_pkce).to be true
      expect(auth.oauth_application_scopes).to include('launch/patient', 'patient/*.rs', 'offline_access')
    end

    it 'uses short access tokens, rotating refresh tokens, and a thirty day refresh lifetime' do
      expect(auth.oauth_access_token_expires_in).to eq(15.minutes)
      expect(auth.oauth_refresh_token_expires_in).to eq(30.days)
      expect(auth.oauth_refresh_token_protection_policy).to eq('rotation')
      expect(auth.oauth_applications_client_secret_hash_column).to eq(:client_secret_hash)
    end

    it 'exposes authorization, token, and revocation routes' do
      expect(auth).to respond_to(:authorize_path, :token_path, :revoke_path)
    end
  end

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

  describe 'WebAuthn verification auditing' do
    let(:auth) { RodauthApp.rodauth.allocate }

    before { allow(auth).to receive(:audit_auth_token) }

    it 'records successful user verification' do
      auth.send(:before_webauthn_auth)

      expect(auth).to have_received(:audit_auth_token).with(
        'webauthn_verification', 'succeeded', outcome: 'success'
      )
    end

    it 'records failed user verification' do
      auth.send(:after_webauthn_auth_failure)

      expect(auth).to have_received(:audit_auth_token).with(
        'webauthn_verification', 'failed', outcome: 'failure'
      )
    end
  end

  describe 'Zitadel professional title helpers' do
    it 'returns an empty role list when Zitadel roles are absent' do
      auth = RodauthApp.rodauth.allocate

      expect(auth.send(:zitadel_role_names, { 'extra' => { 'raw_info' => {} } })).to eq([])
    end

    it 'returns an empty role list when raw OIDC info is absent' do
      auth = RodauthApp.rodauth.allocate

      expect(auth.send(:zitadel_role_names, { 'extra' => {} })).to eq([])
    end

    it 'maps doctor and nurse roles to professional titles' do
      auth = RodauthApp.rodauth.allocate

      expect(auth.send(:zitadel_professional_title_for, zitadel_auth_data(%w[administrator doctor]))).to eq('doctor')
      expect(auth.send(:zitadel_professional_title_for, zitadel_auth_data(%w[member nurse]))).to eq('nurse')
    end

    it 'returns nil when no professional role is present' do
      auth = RodauthApp.rodauth.allocate

      expect(auth.send(:zitadel_professional_title_for, zitadel_auth_data(%w[administrator member]))).to be_nil
    end

    it 'updates a changed professional title' do
      auth = RodauthApp.rodauth.allocate
      person = people(:doctor_jones)
      person.update!(professional_title: nil)

      auth.send(:sync_zitadel_professional_title!, person, zitadel_auth_data(['doctor']))

      expect(person.reload.professional_title).to eq('doctor')
    end

    it 'does nothing when no professional title is present' do
      auth = RodauthApp.rodauth.allocate
      person = people(:doctor_jones)
      person.update!(professional_title: nil)

      auth.send(:sync_zitadel_professional_title!, person, zitadel_auth_data(['member']))

      expect(person.reload.professional_title).to be_nil
    end

    it 'does nothing when the professional title already matches' do
      auth = RodauthApp.rodauth.allocate
      person = people(:doctor_jones)
      allow(person).to receive(:update)

      auth.send(:sync_zitadel_professional_title!, person, zitadel_auth_data(['doctor']))

      expect(person).not_to have_received(:update)
    end

    it 'logs failed professional title updates without raising' do
      auth = RodauthApp.rodauth.allocate
      errors = instance_double(ActiveModel::Errors, full_messages: ['Professional title is invalid'])
      person = instance_double(Person, id: 123, professional_title: nil, errors: errors)
      allow(person).to receive(:update).and_return(false)
      allow(Rails.logger).to receive(:warn)

      auth.send(:sync_zitadel_professional_title!, person, zitadel_auth_data(['doctor']))

      expect(Rails.logger).to have_received(:warn).with(
        '[OIDC] Professional title sync failed for 123: Professional title is invalid'
      )
    end
  end

  def zitadel_auth_data(role_names)
    {
      'extra' => {
        'raw_info' => {
          'urn:zitadel:iam:org:project:roles' => role_names.index_with { {} }
        }
      }
    }
  end
end
