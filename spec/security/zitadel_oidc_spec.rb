# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Zitadel OIDC Enhancements' do # rubocop:disable RSpec/DescribeClass
  let(:rodauth_source) { Rails.root.join('app/misc/rodauth_main.rb').read }
  let(:authentication_source) { Rails.root.join('app/controllers/concerns/authentication.rb').read }

  describe 'RP-initiated logout (single sign-out)' do
    it 'captures the OIDC ID token from session before_logout' do
      expect(rodauth_source).to include('before_logout')
      expect(rodauth_source).to include('session[:oidc_id_token]')
    end

    it 'redirects to the OIDC end_session endpoint after_logout' do
      expect(rodauth_source).to include('after_logout')
      expect(rodauth_source).to include('/oidc/v1/end_session')
    end

    it 'includes id_token_hint and post_logout_redirect_uri in the logout redirect' do
      expect(rodauth_source).to include('id_token_hint')
      expect(rodauth_source).to include('post_logout_redirect_uri')
    end

    it 'skips Zitadel redirect when no OIDC ID token is present (password login)' do
      expect(rodauth_source).to match(/next unless @oidc_id_token_for_logout/)
    end

    it 'uses CGI.escape for safe URL encoding of token and redirect URI' do
      expect(rodauth_source).to include('CGI.escape(@oidc_id_token_for_logout)')
      expect(rodauth_source).to include('CGI.escape(app_url)')
    end

    it 'falls back to request.base_url when APP_URL is not set' do
      expect(rodauth_source).to include("ENV.fetch('APP_URL', request.base_url)")
    end
  end

  describe 'Zitadel claim mapping' do
    it 'reads urn:zitadel:iam:org:project:roles claim from the ID token' do
      expect(rodauth_source).to include("'urn:zitadel:iam:org:project:roles'")
    end

    it 'defines professional title helper methods without household membership authority sync' do
      expect(rodauth_source).to include('def zitadel_professional_title_for(auth_data)')
      expect(rodauth_source).not_to include('def zitadel_membership_role_for(auth_data)')
      expect(rodauth_source).not_to include('sync_zitadel_membership!')
    end

    it 'does not write OIDC roles into the legacy user role column' do
      expect(rodauth_source).not_to include('role: zitadel')
      expect(rodauth_source).not_to include('user.update(role:')
      expect(rodauth_source).not_to include('membership.update(role:')
    end

    it 'creates OIDC users without assigning an authorization role' do
      expect(rodauth_source).to include('User.create!')
      expect(rodauth_source).to include('create_household_for_account!(account_record, person, household: household)')
    end

    it 'logs a warning when professional title sync fails instead of raising' do
      expect(rodauth_source).to include('Rails.logger.warn')
      expect(rodauth_source).to include('Professional title sync failed')
    end

    it 'uses find_by to avoid raising on missing account' do
      expect(rodauth_source).to include('Account.find_by(id: account_id)')
    end
  end

  describe 'MFA delegation to Zitadel via amr claim' do
    it 'reads the amr claim from raw_info to detect Zitadel MFA' do
      expect(rodauth_source).to include("omniauth_auth.dig('extra', 'raw_info', 'amr')")
    end

    it 'sets oidc_mfa_verified session flag when MFA amr values are present' do
      expect(rodauth_source).to include('session[:oidc_mfa_verified]')
      expect(rodauth_source).to include('intersect?(%w[mfa otp u2f hwk swk])')
    end

    it 'checks the session flag (not account identity) in the Authentication concern' do
      expect(authentication_source).to include('session[:oidc_mfa_verified] == true')
    end

    it 'guards should_setup_two_factor? with oidc_authenticated? before the household manager check' do
      oidc_guard_pos = authentication_source.index('return false if oidc_authenticated?')
      manager_check_pos = authentication_source.index('household_manager_requires_two_factor?')
      expect(oidc_guard_pos).to be < manager_check_pos
    end
  end
end
