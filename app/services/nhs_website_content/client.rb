# frozen_string_literal: true

require 'json'
require 'net/http'
require 'uri'

module NhsWebsiteContent
  class Client
    class ApiError < StandardError; end

    SANDBOX_BASE_URL = 'https://sandbox.api.service.nhs.uk/nhs-website-content'
    PRODUCTION_BASE_URL = 'https://api.service.nhs.uk/nhs-website-content'
    CACHE_PREFIX = 'nhs_website_content'
    TIMEOUT_SECONDS = 5
    NETWORK_ERRORS = [
      Net::OpenTimeout,
      Net::ReadTimeout,
      Errno::ECONNREFUSED,
      Errno::ECONNRESET,
      EOFError,
      SocketError,
      OpenSSL::SSL::SSLError
    ].freeze

    def configured?
      sandbox? || api_key.present?
    end

    def list_medicines(category:, page: '1')
      fetch_json(
        cache_key: "#{CACHE_PREFIX}/medicines/index/#{base_url}/#{category}/#{page}",
        path: '/medicines',
        params: { 'category' => category, 'page' => page }
      )
    end

    def get_medicine(slug:, modules: true)
      fetch_json(
        cache_key: "#{CACHE_PREFIX}/medicines/page/#{base_url}/#{slug}/#{modules}",
        path: "/medicines/#{slug}/",
        params: { 'modules' => modules.to_s }
      )
    end

    private

    def fetch_json(cache_key:, path:, params:)
      Rails.cache.fetch(cache_key, expires_in: 12.hours) do
        uri = URI("#{base_url}#{path}")
        uri.query = URI.encode_www_form(params.compact)
        response = perform_request(uri)
        parse_response(response)
      end
    end

    def perform_request(uri)
      build_http_client(uri).request(build_request(uri))
    rescue *NETWORK_ERRORS => e
      raise ApiError, "NHS website API request failed: #{e.message}"
    end

    def parse_response(response)
      raise ApiError, "NHS website API returned #{response.code}" unless response.is_a?(Net::HTTPSuccess)

      JSON.parse(response.body)
    rescue JSON::ParserError => e
      raise ApiError, "NHS website API returned invalid JSON: #{e.message}"
    end

    def sandbox?
      ENV.fetch('NHS_WEBSITE_CONTENT_USE_SANDBOX', 'false') == 'true'
    end

    def base_url
      sandbox? ? SANDBOX_BASE_URL : PRODUCTION_BASE_URL
    end

    def api_key
      ENV.fetch('NHS_WEBSITE_CONTENT_API_KEY', nil)
    end

    def build_http_client(uri)
      Net::HTTP.new(uri.host, uri.port).tap do |http|
        http.use_ssl = true
        http.open_timeout = TIMEOUT_SECONDS
        http.read_timeout = TIMEOUT_SECONDS
      end
    end

    def build_request(uri)
      Net::HTTP::Get.new(uri).tap do |request|
        request['Accept'] = 'application/json'
        request['apikey'] = api_key if api_key.present?
      end
    end
  end
end
