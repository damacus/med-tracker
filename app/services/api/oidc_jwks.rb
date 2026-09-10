# frozen_string_literal: true

require 'net/http'

module Api
  class OidcJwks
    class Error < StandardError; end

    CACHE_TTL = 15.minutes
    NETWORK_ERRORS = [
      Timeout::Error,
      Errno::ECONNREFUSED,
      Errno::ECONNRESET,
      EOFError,
      SocketError,
      OpenSSL::SSL::SSLError
    ].freeze

    def initialize(issuer:, cache: Rails.cache)
      @issuer = issuer.to_s.delete_suffix('/')
      @cache = cache
    end

    def call(options = {})
      cache.delete(jwks_cache_key) if options[:kid_not_found]

      JWT::JWK::Set.new(jwks_document)
    rescue JWT::JWKError => e
      raise Error, e.message
    end

    private

    attr_reader :cache, :issuer

    def jwks_document
      cache.fetch(jwks_cache_key, expires_in: CACHE_TTL) do
        document = fetch_json(jwks_uri)
        signing_keys = Array(document['keys']).select { |key| key['use'].blank? || key['use'] == 'sig' }
        raise Error, 'OIDC signing keys are unavailable' if signing_keys.empty?

        { 'keys' => signing_keys }
      end
    end

    def jwks_uri
      document = cache.fetch(discovery_cache_key, expires_in: CACHE_TTL) do
        fetch_json("#{issuer}/.well-known/openid-configuration")
      end
      raise Error, 'OIDC discovery issuer does not match' unless document['issuer'] == issuer

      uri = URI.parse(document.fetch('jwks_uri'))
      raise Error, 'OIDC JWKS URI must use HTTPS' unless uri.is_a?(URI::HTTPS)

      uri
    rescue KeyError, URI::InvalidURIError
      raise Error, 'OIDC discovery document is invalid'
    end

    def fetch_json(uri)
      uri = URI.parse(uri) unless uri.is_a?(URI)
      response = http_client(uri).request(Net::HTTP::Get.new(uri, 'Accept' => 'application/json'))
      raise Error, "OIDC metadata request returned #{response.code}" unless response.is_a?(Net::HTTPSuccess)

      JSON.parse(response.body)
    rescue JSON::ParserError
      raise Error, 'OIDC metadata response is invalid'
    rescue *NETWORK_ERRORS => e
      raise Error, "OIDC metadata request failed: #{e.message}"
    end

    def http_client(uri)
      Net::HTTP.new(uri.host, uri.port).tap do |http|
        http.use_ssl = uri.is_a?(URI::HTTPS)
        http.open_timeout = 5
        http.read_timeout = 5
      end
    end

    def discovery_cache_key
      "oidc/discovery/#{Digest::SHA256.hexdigest(issuer)}"
    end

    def jwks_cache_key
      "oidc/jwks/#{Digest::SHA256.hexdigest(issuer)}"
    end
  end
end
