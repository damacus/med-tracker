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

    it 'includes id_token_hint in the logout redirect' do
      expect(rodauth_source).to include('id_token_hint')
    end

    it 'includes post_logout_redirect_uri in the logout redirect' do
      expect(rodauth_source).to include('post_logout_redirect_uri')
    end

    it 'skips Zitadel redirect when no OIDC ID token is present (password login)' do
      expect(rodauth_source).to include('@oidc_id_token_for_logout')
      expect(rodauth_source).to match(/next unless @oidc_id_token_for_logout/)
    end

    it 'uses CGI.escape for safe URL encoding of token and redirect URI' do
      expect(rodauth_source).to include('CGI.escape(@oidc_id_token_for_logout)')
      expect(rodauth_source).to include('CGI.escape(app_url)')
    end
  end

  describe 'Zitadel role mapping' do
    it 'reads urn:zitadel:iam:org:project:roles claim from the ID token' do
      expect(rodauth_source).to include("'urn:zitadel:iam:org:project:roles'")
    end

    it 'defines zitadel_role_for helper method' do
      expect(rodauth_source).to include('def zitadel_role_for(auth_data)')
    end

    it 'intersects Zitadel role names with valid User roles' do
      expect(rodauth_source).to include('User.roles.keys & zitadel_roles')
    end

    it 'returns nil when the roles claim is absent to preserve existing roles' do
      expect(rodauth_source).to include("return nil unless raw_info.key?('urn:zitadel:iam:org:project:roles')")
    end

    it 'falls back to :parent when the claim is present but no role matches' do
      expect(rodauth_source).to include(':parent')
      expect(rodauth_source).to match(/valid_roles\.first.*\|\| :parent/)
    end

    it 'applies role mapping on account creation via after_omniauth_create_account' do
      expect(rodauth_source).to include('role: zitadel_role_for(omniauth_auth)')
    end

    it 'stores OIDC ID token via omniauth_auth in session on every OIDC login' do
      expect(rodauth_source).to include("omniauth_auth&.dig('credentials', 'id_token')")
      expect(rodauth_source).to include('session[:oidc_id_token] = id_token')
    end

    it 'only syncs role when claim is present (nil-guard prevents silent downgrade)' do
      expect(rodauth_source).to include('user.update(role: new_role) if new_role &&')
    end
  end

  describe '2FA bypass for OIDC users' do
    it 'defines oidc_authenticated? helper in the Authentication concern' do
      expect(authentication_source).to include('def oidc_authenticated?')
    end

    it 'checks AccountIdentity for OIDC provider in oidc_authenticated?' do
      expected = "AccountIdentity.exists?(account_id: current_account.id, provider: 'oidc')"
      expect(authentication_source).to include(expected)
    end

    it 'guards should_setup_two_factor? with oidc_authenticated? check' do
      expect(authentication_source).to include('return false if oidc_authenticated?')
    end

    it 'places the OIDC guard before the role check in should_setup_two_factor?' do
      oidc_guard_pos = authentication_source.index('return false if oidc_authenticated?')
      roles_check_pos = authentication_source.index('ROLES_REQUIRING_2FA.include?')
      expect(oidc_guard_pos).to be < roles_check_pos
    end
  end
end
