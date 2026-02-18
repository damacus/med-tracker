# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'OIDC Security' do # rubocop:disable RSpec/DescribeClass
  describe 'OIDC-SEC-009: Client secret stored securely' do
    it 'does not expose client secret in Rodauth configuration source' do
      rodauth_file = Rails.root.join('app/misc/rodauth_main.rb').read
      expect(rodauth_file).not_to match(/client_secret.*=.*['"][a-zA-Z0-9]{10,}['"]/)
    end

    it 'reads client secret from credentials or environment variables only' do
      rodauth_file = Rails.root.join('app/misc/rodauth_main.rb').read
      expect(rodauth_file).to include('credentials.dig(:oidc,')
      expect(rodauth_file).to include(':client_secret)')
      expect(rodauth_file).to include("ENV.fetch('OIDC_CLIENT_SECRET'")
    end

    it 'does not have hardcoded secrets in source files' do
      expect(OidcSecurity.secret_not_in_source?).to be true
    end
  end

  describe 'OIDC-SEC-008: Authorization code flow' do
    it 'configures response_type as code (not implicit)' do
      rodauth_file = Rails.root.join('app/misc/rodauth_main.rb').read
      expect(rodauth_file).to include('response_type: :code')
    end

    it 'does not use implicit flow' do
      rodauth_file = Rails.root.join('app/misc/rodauth_main.rb').read
      expect(rodauth_file).not_to include('response_type: :token')
      expect(rodauth_file).not_to include("response_type: 'token'")
    end
  end

  describe 'OIDC-SEC-001: Token signature verification via JWKS' do
    it 'enables discovery mode for automatic JWKS fetching' do
      rodauth_file = Rails.root.join('app/misc/rodauth_main.rb').read
      expect(rodauth_file).to include('discovery: true')
    end
  end

  describe 'OIDC-SEC-004: Issuer validation' do
    it 'configures issuer in provider options' do
      rodauth_file = Rails.root.join('app/misc/rodauth_main.rb').read
      expect(rodauth_file).to include('issuer: oidc_issuer')
    end
  end

  describe 'OIDC-SEC-005: State parameter CSRF protection' do
    it 'has omniauth feature enabled which provides state parameter' do
      rodauth_file = Rails.root.join('app/misc/rodauth_main.rb').read
      expect(rodauth_file).to include(':omniauth')
    end
  end

  describe 'OIDC-SEC-010: Redirect URI validation' do
    it 'configures explicit redirect_uri in client options' do
      rodauth_file = Rails.root.join('app/misc/rodauth_main.rb').read
      expect(rodauth_file).to include('redirect_uri:')
    end

    it 'uses /auth/oidc/callback path' do
      rodauth_file = Rails.root.join('app/misc/rodauth_main.rb').read
      expect(rodauth_file).to include('/auth/oidc/callback')
    end
  end

  describe 'OIDC-SEC-007: Access token not stored long-term' do
    it 'stores only provider and UID in account_identities table' do
      expect(ActiveRecord::Base.connection.table_exists?(:account_identities)).to be true

      columns = ActiveRecord::Base.connection.columns(:account_identities).map(&:name)
      expect(columns).to include('provider')
      expect(columns).to include('uid')
      expect(columns).not_to include('access_token')
      expect(columns).not_to include('refresh_token')
    end
  end

  describe 'OIDC-SEC-011: Account hijacking prevention via email verification' do
    it 'marks OIDC accounts as verified to prevent hijacking' do
      rodauth_file = Rails.root.join('app/misc/rodauth_main.rb').read
      expect(rodauth_file).to include('verify_account_login_status: :verified')
    end
  end

  describe 'OIDC-SEC-012: Session fixation prevention' do
    it 'has active_sessions feature enabled for session management' do
      rodauth_file = Rails.root.join('app/misc/rodauth_main.rb').read
      expect(rodauth_file).to include(':active_sessions')
    end
  end

  describe 'OIDC configuration uses openid_connect gem' do
    it 'uses omniauth_openid_connect gem instead of google-specific gem' do
      gemfile = Rails.root.join('Gemfile').read
      expect(gemfile).to include('omniauth_openid_connect')
      expect(gemfile).not_to include('omniauth-google-oauth2')
    end

    it 'configures provider as openid_connect with name :oidc' do
      rodauth_file = Rails.root.join('app/misc/rodauth_main.rb').read
      expect(rodauth_file).to include('omniauth_provider :openid_connect')
      expect(rodauth_file).to include('name: :oidc')
    end

    it 'requests openid, email, and profile scopes' do
      rodauth_file = Rails.root.join('app/misc/rodauth_main.rb').read
      expect(rodauth_file).to include('scope: %i[openid email profile]')
    end

    it 'uses sub as uid_field' do
      rodauth_file = Rails.root.join('app/misc/rodauth_main.rb').read
      expect(rodauth_file).to include("uid_field: 'sub'")
    end
  end

  describe 'OIDC-SEC-002: Token expiration prevents replay attacks' do
    it 'uses openid_connect gem which validates token expiration automatically' do
      rodauth_file = Rails.root.join('app/misc/rodauth_main.rb').read
      expect(rodauth_file).to include('omniauth_provider :openid_connect')
    end

    it 'does not store tokens long-term, forcing expiration validation' do
      expect(ActiveRecord::Base.connection.table_exists?(:account_identities)).to be true

      columns = ActiveRecord::Base.connection.columns(:account_identities).map(&:name)
      expect(columns).not_to include('access_token')
      expect(columns).not_to include('refresh_token')
    end
  end

  describe 'OIDC-SEC-003: Audience claim validation prevents token misuse' do
    it 'configures client identifier which enables audience validation' do
      rodauth_file = Rails.root.join('app/misc/rodauth_main.rb').read
      expect(rodauth_file).to include('identifier: oidc_client_id')
    end

    it 'uses openid_connect gem which validates audience claim automatically' do
      rodauth_file = Rails.root.join('app/misc/rodauth_main.rb').read
      expect(rodauth_file).to include('omniauth_provider :openid_connect')
    end
  end

  describe 'OIDC-SEC-006: Nonce prevents token replay attacks' do
    it 'uses openid_connect gem which handles nonce automatically' do
      rodauth_file = Rails.root.join('app/misc/rodauth_main.rb').read
      expect(rodauth_file).to include('omniauth_provider :openid_connect')
    end

    it 'enables omniauth feature which provides nonce support' do
      rodauth_file = Rails.root.join('app/misc/rodauth_main.rb').read
      expect(rodauth_file).to include(':omniauth')
    end
  end
end
