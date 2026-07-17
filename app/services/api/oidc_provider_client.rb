# frozen_string_literal: true

module Api
  class OidcProviderClient
    class Error < StandardError; end

    DISCOVERY_CACHE_TTL = 5.minutes
    JWKS_CACHE_TTL = 5.minutes
    HTTP_TIMEOUT = 10
    HTTP_OPEN_TIMEOUT = 5
    ALLOWED_SIGNING_ALGORITHMS = %w[RS256 RS384 RS512 PS256 PS384 PS512 ES256 ES384 ES512].freeze

    def initialize(connection: nil, cache: Rails.cache)
      @connection = connection || Faraday.new
      @cache = cache
    end

    def exchange_code(authorization_code:, code_verifier:, redirect_uri:)
      validate_configuration!
      validate_redirect_uri!(redirect_uri)
      response = token_request(authorization_code, code_verifier, redirect_uri)
      payload = response_payload(response)
      payload.fetch('id_token').presence || raise(Error)
    rescue Faraday::Error, JSON::ParserError, KeyError, URI::InvalidURIError
      raise Error
    end

    def decode_id_token(id_token)
      configuration = provider_configuration
      algorithms = Array(configuration['id_token_signing_alg_values_supported']) & ALLOWED_SIGNING_ALGORITHMS
      raise Error if algorithms.empty?

      payload, = JWT.decode(id_token, nil, true, decode_options(configuration, algorithms))
      payload
    rescue Faraday::Error, JSON::ParserError, JWT::DecodeError, KeyError, URI::InvalidURIError
      raise Error
    end

    private

    attr_reader :connection, :cache

    def token_request(authorization_code, code_verifier, redirect_uri)
      connection.post(provider_configuration.fetch('token_endpoint')) do |request|
        configure_request(request)
        request.headers['Content-Type'] = 'application/x-www-form-urlencoded'
        request.body = URI.encode_www_form(token_parameters(authorization_code, code_verifier, redirect_uri))
      end
    end

    def token_parameters(authorization_code, code_verifier, redirect_uri)
      {
        code: authorization_code,
        code_verifier: code_verifier,
        client_id: client_id,
        redirect_uri: redirect_uri,
        grant_type: 'authorization_code'
      }
    end

    def decode_options(configuration, algorithms)
      {
        algorithms: algorithms,
        jwks: ->(_options) { jwks(configuration.fetch('jwks_uri')) },
        iss: issuer,
        verify_iss: true,
        aud: client_id,
        verify_aud: true,
        verify_expiration: true,
        verify_iat: true,
        required_claims: %w[iss aud exp iat nonce sub]
      }
    end

    def provider_configuration
      @provider_configuration ||= cache.fetch(discovery_cache_key, expires_in: DISCOVERY_CACHE_TTL) do
        configuration = get_json(discovery_url)
        raise Error unless configuration['issuer'] == issuer

        validate_https_url!(configuration.fetch('token_endpoint'))
        validate_https_url!(configuration.fetch('jwks_uri'))
        configuration
      end
    rescue KeyError
      raise Error
    end

    def jwks(uri)
      cache.fetch("api/oidc/jwks/#{Digest::SHA256.hexdigest(uri)}", expires_in: JWKS_CACHE_TTL) do
        payload = get_json(uri)
        raise Error unless payload['keys'].is_a?(Array) && payload['keys'].present?

        payload
      end
    end

    def get_json(url)
      response = connection.get(url) do |request|
        configure_request(request)
      end
      response_payload(response)
    end

    def configure_request(request)
      request.options.timeout = HTTP_TIMEOUT
      request.options.open_timeout = HTTP_OPEN_TIMEOUT
    end

    def response_payload(response)
      raise Error unless response.success?

      JSON.parse(response.body)
    end

    def validate_configuration!
      raise Error if issuer.blank? || discovery_url.blank? || client_id.blank? || redirect_uris.empty?

      validate_https_url!(issuer)
      validate_https_url!(discovery_url)
    end

    def validate_redirect_uri!(redirect_uri)
      raise Error unless redirect_uris.include?(redirect_uri.to_s)

      uri = URI.parse(redirect_uri)
      raise Error if uri.scheme.blank?
      raise Error if uri.scheme == 'http' && !Rails.env.test?
    end

    def validate_https_url!(value)
      uri = URI.parse(value)
      raise Error unless uri.is_a?(URI::HTTP)
      raise Error if uri.scheme != 'https' && !Rails.env.test?
      raise Error if value == issuer && (uri.query.present? || uri.fragment.present?)
    end

    def issuer
      @issuer ||= ENV.fetch('OIDC_ISSUER_URL', nil).presence ||
                  Rails.application.credentials.dig(:oidc, :issuer_url).to_s
    end

    def discovery_url
      @discovery_url ||= ENV.fetch('OIDC_DISCOVERY_URL', nil).presence ||
                         Rails.application.credentials.dig(:oidc, :discovery_url).presence ||
                         "#{issuer.delete_suffix('/')}/.well-known/openid-configuration"
    end

    def client_id
      @client_id ||= ENV.fetch('OIDC_MOBILE_CLIENT_ID', nil).presence ||
                     Rails.application.credentials.dig(:oidc, :mobile_client_id).to_s
    end

    def redirect_uris
      @redirect_uris ||= Array(configured_redirect_uris).flat_map { it.to_s.split(',') }
                                                        .map(&:strip).compact_blank.uniq
    end

    def configured_redirect_uris
      ENV.fetch('OIDC_MOBILE_REDIRECT_URIS', nil).presence ||
        ENV.fetch('OIDC_MOBILE_REDIRECT_URI', nil).presence ||
        Rails.application.credentials.dig(:oidc, :mobile_redirect_uris).presence ||
        Rails.application.credentials.dig(:oidc, :mobile_redirect_uri)
    end

    def discovery_cache_key
      value = "#{issuer}\0#{discovery_url}"
      "api/oidc/discovery/#{Digest::SHA256.hexdigest(value)}"
    end
  end
end
