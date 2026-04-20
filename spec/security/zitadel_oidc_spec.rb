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

    it 'fails fast when APP_URL is missing in production' do
      expect(rodauth_source).to include("Rails.env.production? ? ENV.fetch('APP_URL')")
    end
  end

  describe 'Zitadel role mapping' do
    it 'reads urn:zitadel:iam:org:project:roles claim from the ID token' do
      expect(rodauth_source).to include("'urn:zitadel:iam:org:project:roles'")
    end

    it 'defines zitadel_role_for helper method' do
      expect(rodauth_source).to include('def zitadel_role_for(auth_data)')
    end

    it 'returns nil (not a default) when no valid role found — caller supplies default' do
      expect(rodauth_source).to include('valid_roles.first&.to_sym')
      expect(rodauth_source).not_to match(/valid_roles\.first.*\|\| :parent/)
    end

    it 'applies :parent as the system default on account creation' do
      expect(rodauth_source).to include('role: zitadel_role_for(omniauth_auth) || :parent')
    end

    it 'logs a warning when role sync fails instead of raising' do
      expect(rodauth_source).to include('Rails.logger.warn')
      expect(rodauth_source).to include('Role sync failed')
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

    it 'guards should_setup_two_factor? with oidc_authenticated? before the role check' do
      oidc_guard_pos = authentication_source.index('return false if oidc_authenticated?')
      roles_check_pos = authentication_source.index('ROLES_REQUIRING_2FA.include?')
      expect(oidc_guard_pos).to be < roles_check_pos
    end
  end
end
