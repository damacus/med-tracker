module DemoBaseline
  class PublicMobileClients
    class MismatchedClientError < StandardError; end

    ANDROID_CLIENT_IDS = %w[
      io.damacus.medtracker
      io.damacus.medtracker.debug
      io.damacus.medtracker.staging
    ].freeze
    REQUIRED_ATTRIBUTES = %i[
      name client_id redirect_uri scopes client_kind token_endpoint_auth_method
      account_id client_secret client_secret_hash
    ].freeze
    DEFINITIONS = (
      ANDROID_CLIENT_IDS.map do |client_id|
        {
          name: 'MedTracker Android', client_id:, redirect_uri: "#{client_id}:/oauth2redirect",
          scopes: 'medtracker offline_access', client_kind: 'mobile', token_endpoint_auth_method: 'none',
          account_id: nil, client_secret: nil, client_secret_hash: nil
        }
      end + [{
        name: 'MedTracker iOS Staging', client_id: 'medtracker-ios-staging',
        redirect_uri: 'io.damacus.medtracker.staging:/oauth2redirect',
        scopes: 'medtracker offline_access', client_kind: 'mobile', token_endpoint_auth_method: 'none',
        account_id: nil, client_secret: nil, client_secret_hash: nil
      }]
    ).freeze

    def self.register!
      DEFINITIONS.each do |attributes|
        client = OauthApplication.find_by(client_id: attributes.fetch(:client_id))
        if client
          unless matches?(client, attributes)
            raise MismatchedClientError, 'first-party OAuth client differs from registration'
          end
        else
          OauthApplication.create!(attributes)
        end
      end
    end

    def self.valid_baseline?
      OauthApplication.count == DEFINITIONS.length && !OauthGrant.exists? && DEFINITIONS.all? do |attributes|
        client = OauthApplication.find_by(client_id: attributes.fetch(:client_id))
        client && matches?(client, attributes)
      end
    end

    def self.matches?(client, attributes)
      REQUIRED_ATTRIBUTES.all? { |key| client.public_send(key) == attributes.fetch(key) }
    end
    private_class_method :matches?
  end
end
