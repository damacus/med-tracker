# frozen_string_literal: true

require 'json'
require 'net/http'
require 'uri'

module OpenFoodFacts
  class Client
    class ApiError < StandardError; end

    BASE_URL = 'https://world.openfoodfacts.org'
    CACHE_PREFIX = 'open_food_facts'
    TIMEOUT_SECONDS = 5
    DEFAULT_FIELDS = %w[product_name brands quantity categories_tags_en image_url].freeze
    DEFAULT_SEARCH_PAGE_SIZE = 10
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

    def search_products(query, page_size: DEFAULT_SEARCH_PAGE_SIZE)
      normalized_query = query.to_s.strip
      return [] if normalized_query.blank?

      Rails.cache.fetch(search_cache_key(normalized_query, page_size), expires_in: 1.hour) do
        uri = URI("#{BASE_URL}/cgi/search.pl")
        uri.query = URI.encode_www_form(
          'search_terms' => normalized_query,
          'search_simple' => '1',
          'action' => 'process',
          'json' => '1',
          'page_size' => page_size.to_s
        )
        response = perform_request(uri)
        parse_search_response(response)
      end
    end

    private

    def cache_key(barcode, fields)
      "#{CACHE_PREFIX}/product/#{barcode}/#{Array(fields).join(',')}"
    end

    def search_cache_key(query, page_size)
      "#{CACHE_PREFIX}/search/#{query.downcase}/#{page_size}"
    end

    def perform_request(uri)
      build_http_client(uri).request(build_request(uri))
    rescue *NETWORK_ERRORS => e
      raise ApiError, "Open Food Facts API request failed: #{e.message}"
    end

    def parse_response(response)
      raise ApiError, "Open Food Facts API returned #{response.code}" unless response.is_a?(Net::HTTPSuccess)

      payload = JSON.parse(response.body)
      return nil unless payload['status'] == 1

      payload
    rescue JSON::ParserError => e
      raise ApiError, "Open Food Facts API returned invalid JSON: #{e.message}"
    end

    def parse_search_response(response)
      raise ApiError, "Open Food Facts API returned #{response.code}" unless response.is_a?(Net::HTTPSuccess)

      payload = JSON.parse(response.body)
      Array(payload['products'])
    rescue JSON::ParserError => e
      raise ApiError, "Open Food Facts API returned invalid JSON: #{e.message}"
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
