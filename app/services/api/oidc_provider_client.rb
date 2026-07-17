# frozen_string_literal: true

require 'ipaddr'

module Api
  class OidcProviderEndpointValidator
    UNSAFE_ADDRESS_RANGES = %w[
      0.0.0.0/8
      10.0.0.0/8
      100.64.0.0/10
      127.0.0.0/8
      169.254.0.0/16
      172.16.0.0/12
      192.0.0.0/24
      192.0.2.0/24
      192.168.0.0/16
      198.18.0.0/15
      198.51.100.0/24
      203.0.113.0/24
      224.0.0.0/4
      240.0.0.0/4
      ::/128
      ::1/128
      fc00::/7
      fe80::/10
      ff00::/8
      2001:db8::/32
    ].map { IPAddr.new(it) }.freeze

    def initialize(issuer:, configured_origins:, resolver:, error_class:)
      @issuer = issuer
      @configured_origins = configured_origins
      @resolver = resolver
      @error_class = error_class
    end

    def validate!(value)
      uri = endpoint_uri(value)
      raise error_class unless allowed_origins.include?(origin(uri))

      validate_resolved_host!(uri)
    rescue URI::InvalidURIError
      raise error_class
    end

    def allowed_origins
      @allowed_origins ||= configured_origin_values.map { normalized_origin(it) }.uniq
    rescue URI::InvalidURIError
      raise error_class
    end

    private

    attr_reader :issuer, :configured_origins, :resolver, :error_class

    def configured_origin_values
      values = configured_origins.presence || origin(URI.parse(issuer))
      Array(values).flat_map { it.to_s.split(',') }.map(&:strip).compact_blank
    end

    def normalized_origin(value)
      uri = endpoint_uri(value)
      raise error_class if uri.path.present? && uri.path != '/'
      raise error_class if uri.query.present? || uri.fragment.present?

      origin(uri)
    end

    def endpoint_uri(value)
      raise error_class unless value.is_a?(String)

      uri = URI.parse(value)
      raise error_class unless uri.is_a?(URI::HTTPS) && uri.host.present?
      raise error_class if uri.userinfo.present?

      uri
    end

    def validate_resolved_host!(uri)
      addresses = literal_or_resolved_addresses(uri)
      raise error_class if addresses.empty? || addresses.any? { unsafe_address?(it) }
    rescue SocketError, SystemCallError, IPAddr::InvalidAddressError
      raise error_class
    end

    def literal_or_resolved_addresses(uri)
      [IPAddr.new(uri.host)]
    rescue IPAddr::InvalidAddressError
      resolver.getaddrinfo(uri.host, uri.port, nil, :STREAM).map { IPAddr.new(it.ip_address) }
    end

    def unsafe_address?(address)
      address = address.native if address.ipv4_mapped?
      UNSAFE_ADDRESS_RANGES.any? { it.include?(address) }
    end

    def origin(uri)
      default_port = uri.scheme == 'https' ? 443 : 80
      port = uri.port == default_port ? nil : uri.port
      "#{uri.scheme}://#{uri.host}#{':' if port}#{port}"
    end
  end

  class OidcProviderClient
    class Error < StandardError; end

    DISCOVERY_CACHE_TTL = 5.minutes
    JWKS_CACHE_TTL = 5.minutes
    HTTP_TIMEOUT = 10
    HTTP_OPEN_TIMEOUT = 5
    ALLOWED_SIGNING_ALGORITHMS = %w[RS256 RS384 RS512 PS256 PS384 PS512 ES256 ES384 ES512].freeze
    def initialize(connection: nil, cache: Rails.cache, resolver: Addrinfo)
      @connection = connection || Faraday.new
      @cache = cache
      @resolver = resolver
    end

    def exchange_code(authorization_code:, code_verifier:, redirect_uri:)
      validate_configuration!
      validate_redirect_uri!(redirect_uri)
      response = token_request(authorization_code, code_verifier, redirect_uri)
      payload = response_payload(response)
      id_token = payload.fetch('id_token')
      raise Error unless id_token.is_a?(String) && id_token.present?

      id_token
    rescue Faraday::Error, JSON::ParserError, KeyError, URI::InvalidURIError
      raise Error
    end

    def decode_id_token(id_token)
      configuration = provider_configuration
      algorithms = Array(configuration['id_token_signing_alg_values_supported']) & ALLOWED_SIGNING_ALGORITHMS
      raise Error if algorithms.empty?

      validate_claims_set_shape!(id_token)
      payload = verified_payload(id_token, configuration, algorithms)
      raise Error unless payload.is_a?(Hash)

      validate_authorized_party!(payload)
      payload
    rescue Faraday::Error, JSON::ParserError, JWT::DecodeError, KeyError, URI::InvalidURIError
      raise Error
    end

    private

    attr_reader :connection, :cache, :resolver

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
        fetch_provider_configuration
      end
    rescue KeyError
      raise Error
    end

    def fetch_provider_configuration
      configuration = get_json(discovery_url)
      raise Error unless configuration['issuer'] == issuer

      endpoint_validator.validate!(configuration.fetch('token_endpoint'))
      endpoint_validator.validate!(configuration.fetch('jwks_uri'))
      validate_signing_algorithms!(configuration.fetch('id_token_signing_alg_values_supported'))
      configuration
    end

    def jwks(uri)
      cache.fetch("api/oidc/jwks/#{Digest::SHA256.hexdigest(uri)}", expires_in: JWKS_CACHE_TTL) do
        payload = get_json(uri)
        keys = payload['keys']
        raise Error unless keys.is_a?(Array) && keys.present? && keys.all?(Hash)

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

      payload = JSON.parse(response.body)
      raise Error unless payload.is_a?(Hash)

      payload
    end

    def validate_claims_set_shape!(id_token)
      payload, = JWT.decode(id_token, nil, false)
      raise Error unless payload.is_a?(Hash)
    end

    def verified_payload(id_token, configuration, algorithms)
      payload, = JWT.decode(id_token, nil, true, decode_options(configuration, algorithms))
      payload
    rescue TypeError, NoMethodError
      raise Error
    end

    def validate_authorized_party!(payload)
      audiences = payload.fetch('aud')
      return if audiences == client_id
      return if audiences == [client_id]
      return if valid_multiple_audiences?(audiences, payload['azp'])

      raise Error
    end

    def valid_multiple_audiences?(audiences, authorized_party)
      audiences.is_a?(Array) && audiences.many? && audiences.all?(String) &&
        audiences.uniq.length == audiences.length && audiences.include?(client_id) && authorized_party == client_id
    end

    def validate_signing_algorithms!(algorithms)
      raise Error unless algorithms.is_a?(Array) && algorithms.present? && algorithms.all?(String)
    end

    def configured_allowed_endpoint_origins
      ENV.fetch('OIDC_ALLOWED_ENDPOINT_ORIGINS', nil).presence ||
        Rails.application.credentials.dig(:oidc, :allowed_endpoint_origins)
    end

    def endpoint_validator
      @endpoint_validator ||= OidcProviderEndpointValidator.new(
        issuer: issuer,
        configured_origins: configured_allowed_endpoint_origins,
        resolver: resolver,
        error_class: Error
      )
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
      value = "#{issuer}\0#{discovery_url}\0#{endpoint_validator.allowed_origins.join(',')}"
      "api/oidc/discovery/#{Digest::SHA256.hexdigest(value)}"
    end
  end
end
