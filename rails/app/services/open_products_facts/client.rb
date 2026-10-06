# frozen_string_literal: true

require 'json'
require 'net/http'
require 'uri'

module OpenProductsFacts
  class Client
    class ApiError < StandardError; end

    BASE_URL = 'https://world.openproductsfacts.org'
    CACHE_PREFIX = 'open_products_facts'
    TIMEOUT_SECONDS = 5
    DEFAULT_FIELDS = %w[product_name generic_name brands quantity categories_tags_en].freeze
    NETWORK_ERRORS = [
      Net::OpenTimeout,
      Net::ReadTimeout,
      Errno::ECONNREFUSED,
      Errno::ECONNRESET,
      EOFError,
      SocketError,
      OpenSSL::SSL::SSLError
    ].freeze

    def product(barcode, fields: DEFAULT_FIELDS)
      normalized = BarcodeCatalogEntry.normalize_gtin(barcode)
      return nil unless normalized.match?(/\A\d{8,14}\z/)

      Rails.cache.fetch(cache_key(normalized, fields), expires_in: 12.hours) do
        uri = URI("#{BASE_URL}/api/v2/product/#{normalized}.json")
        uri.query = URI.encode_www_form('fields' => Array(fields).join(','))
        response = perform_request(uri)
        parse_response(response)
      end
    end

    private

    def cache_key(barcode, fields)
      "#{CACHE_PREFIX}/product/#{barcode}/#{Array(fields).join(',')}"
    end

    def perform_request(uri)
      build_http_client(uri).request(build_request(uri))
    rescue *NETWORK_ERRORS => e
      raise ApiError, "Open Products Facts API request failed: #{e.message}"
    end

    def parse_response(response)
      raise ApiError, "Open Products Facts API returned #{response.code}" unless response.is_a?(Net::HTTPSuccess)

      payload = JSON.parse(response.body)
      return nil unless payload['status'] == 1

      payload
    rescue JSON::ParserError => e
      raise ApiError, "Open Products Facts API returned invalid JSON: #{e.message}"
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
        request['User-Agent'] = user_agent
      end
    end

    def user_agent
      "#{app_name}/#{app_version} (#{contact_email})"
    end

    def app_name
      ENV.fetch('OPEN_FOOD_FACTS_APP_NAME', 'MedTracker')
    end

    def app_version
      ENV.fetch('OPEN_FOOD_FACTS_APP_VERSION', '1.0')
    end

    def contact_email
      ENV.fetch('OPEN_FOOD_FACTS_CONTACT_EMAIL', 'support@medtracker.app')
    end
  end
end
