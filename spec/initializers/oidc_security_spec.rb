# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'OIDC Security Initializer' do # rubocop:disable RSpec/DescribeClass
  describe 'OidcSecurity module' do
    describe '.configured?' do
      it 'returns false when no OIDC credentials are set' do
        allow(Rails.application.credentials).to receive(:dig).and_return(nil)
        allow(ENV).to receive(:fetch).with('OIDC_CLIENT_ID', nil).and_return(nil)
        allow(ENV).to receive(:fetch).with('OIDC_ISSUER_URL', nil).and_return(nil)

        expect(OidcSecurity.configured?).to be false
      end

      it 'returns true when environment variables are set' do
        allow(Rails.application.credentials).to receive(:dig).and_return(nil)
        allow(ENV).to receive(:fetch).and_call_original
        allow(ENV).to receive(:fetch).with('OIDC_CLIENT_ID', nil).and_return('test-client-id')
        allow(ENV).to receive(:fetch).with('OIDC_ISSUER_URL', nil).and_return('https://issuer.example.com')

        expect(OidcSecurity.configured?).to be true
      end

      it 'returns true when Rails credentials are set' do
        allow(Rails.application.credentials).to receive(:dig).with(:oidc, :client_id).and_return('cred-client-id')
        allow(Rails.application.credentials).to receive(:dig).with(:oidc, :issuer_url).and_return('https://issuer.example.com')
        allow(Rails.application.credentials).to receive(:dig).with(:oidc, :client_secret).and_return('cred-secret')

        expect(OidcSecurity.configured?).to be true
      end
    end

    describe '.validate_issuer_url!' do
      it 'accepts valid HTTPS URLs' do
        expect { OidcSecurity.validate_issuer_url!('https://auth.example.com') }.not_to raise_error
      end

      it 'accepts HTTP URLs for localhost' do
        expect { OidcSecurity.validate_issuer_url!('http://localhost:8080') }.not_to raise_error
      end

      it 'accepts HTTP for 127.0.0.1' do
        expect { OidcSecurity.validate_issuer_url!('http://127.0.0.1:8080') }.not_to raise_error
      end

      it 'accepts HTTP for ::1' do
        expect { OidcSecurity.validate_issuer_url!('http://[::1]:8080') }.not_to raise_error
      end

      it 'rejects non-HTTPS URLs in non-localhost contexts' do
        expect { OidcSecurity.validate_issuer_url!('http://auth.example.com') }
          .to raise_error(OidcSecurity::ConfigurationError, /HTTPS/)
      end

      it 'rejects blank URLs' do
        expect { OidcSecurity.validate_issuer_url!('') }
          .to raise_error(OidcSecurity::ConfigurationError, /blank/)
      end

      it 'rejects malformed URLs' do
        expect { OidcSecurity.validate_issuer_url!('not-a-url') }
          .to raise_error(OidcSecurity::ConfigurationError, /Invalid/)
      end
    end

    describe '.secret_not_in_source?' do
      it 'returns true when no hardcoded secrets are found' do
        expect(OidcSecurity.secret_not_in_source?).to be true
      end
    end

    describe '.validate_redirect_uri!' do
      it 'accepts a valid HTTPS redirect URI' do
        expect { OidcSecurity.validate_redirect_uri!('https://app.example.com/auth/oidc/callback') }.not_to raise_error
      end

      it 'accepts HTTP for localhost' do
        expect { OidcSecurity.validate_redirect_uri!('http://localhost:3000/auth/oidc/callback') }.not_to raise_error
      end

      it 'accepts HTTP for 127.0.0.1' do
        expect { OidcSecurity.validate_redirect_uri!('http://127.0.0.1:3000/auth/oidc/callback') }.not_to raise_error
      end

      it 'rejects non-HTTPS URIs for non-localhost' do
        expect { OidcSecurity.validate_redirect_uri!('http://evil.example.com/callback') }
          .to raise_error(OidcSecurity::ConfigurationError, /HTTPS/)
      end

      it 'rejects blank redirect URI' do
        expect { OidcSecurity.validate_redirect_uri!('') }
          .to raise_error(OidcSecurity::ConfigurationError, /blank/)
      end

      it 'rejects malformed redirect URI' do
        expect { OidcSecurity.validate_redirect_uri!('not-a-uri') }
          .to raise_error(OidcSecurity::ConfigurationError, /Invalid/)
      end
    end
  end
end
