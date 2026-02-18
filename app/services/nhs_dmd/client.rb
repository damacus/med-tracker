# frozen_string_literal: true

require 'net/http'
require 'uri'
require 'json'

module NhsDmd
  class Client
    class ApiError < StandardError; end

    BASE_URL = 'https://ontology.nhs.uk/production1/fhir'
    TOKEN_URL = 'https://ontology.nhs.uk/authorisation/auth/realms/nhs-digital-terminology/protocol/openid-connect/token'
    DEFAULT_COUNT = 20
    TIMEOUT_SECONDS = 10

    VMP_VALUE_SET = 'https://dmd.nhs.uk/ValueSet/VMP'
    AMP_VALUE_SET = 'https://dmd.nhs.uk/ValueSet/AMP'

    def configured?
      client_id.present? && client_secret.present?
    end

    def search(query, count: DEFAULT_COUNT)
      return [] if query.blank?
      return [] unless configured?

      vmp_results = fetch_value_set(VMP_VALUE_SET, query, count: count)
      amp_results = fetch_value_set(AMP_VALUE_SET, query, count: count)

      (vmp_results + amp_results).uniq { |r| r[:code] }
    end

    private

    def fetch_value_set(value_set_url, query, count:)
      uri = build_uri(value_set_url, query, count)
      response = perform_request(uri)
      parse_response(response)
    end

    def build_uri(value_set_url, query, count)
      uri = URI("#{BASE_URL}/ValueSet/$expand")
      uri.query = URI.encode_www_form(
        'url' => value_set_url,
        'count' => count.to_s,
        'filter' => query
      )
      uri
    end

    def perform_request(uri)
      http = Net::HTTP.new(uri.host, uri.port)
      http.use_ssl = true
      http.open_timeout = TIMEOUT_SECONDS
      http.read_timeout = TIMEOUT_SECONDS

      request = Net::HTTP::Get.new(uri)
      request['Accept'] = 'application/json'
      request['Authorization'] = "Bearer #{access_token}" if authenticated?

      http.request(request)
    rescue Net::OpenTimeout, Net::ReadTimeout, Errno::ECONNREFUSED => e
      raise ApiError, "NHS dm+d API request failed: #{e.message}"
    end

    def parse_response(response)
      raise ApiError, "NHS dm+d API returned #{response.code}" unless response.is_a?(Net::HTTPSuccess)

      body = JSON.parse(response.body)
      contains = body.dig('expansion', 'contains') || []
      contains.map { |item| map_item(item) }
    rescue JSON::ParserError => e
      raise ApiError, "NHS dm+d API returned invalid JSON: #{e.message}"
    end

    def map_item(item)
      {
        code: item['code'],
        display: item['display'],
        system: item['system'],
        concept_class: extract_concept_class(item)
      }
    end

    def extract_concept_class(item)
      extensions = item['extension'] || []
      comment_ext = extensions.find do |ext|
        ext['url'] == 'http://hl7.org/fhir/StructureDefinition/valueset-concept-comments'
      end
      comment_ext&.dig('valueString')
    end

    def authenticated?
      client_id.present? && client_secret.present?
    end

    def access_token
      @access_token ||= fetch_access_token
    end

    def fetch_access_token
      uri = URI(TOKEN_URL)
      response = Net::HTTP.post_form(uri, {
                                       'grant_type' => 'client_credentials',
                                       'client_id' => client_id,
                                       'client_secret' => client_secret
                                     })

      raise ApiError, "Failed to obtain NHS dm+d access token: #{response.code}" unless response.is_a?(Net::HTTPSuccess)

      JSON.parse(response.body)['access_token']
    rescue Net::OpenTimeout, Net::ReadTimeout => e
      raise ApiError, "NHS dm+d token request failed: #{e.message}"
    end

    def client_id
      ENV.fetch('NHS_DMD_CLIENT_ID', nil)
    end

    def client_secret
      ENV.fetch('NHS_DMD_CLIENT_SECRET', nil)
    end
  end
end
