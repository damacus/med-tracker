# frozen_string_literal: true

require 'net/http'
require 'uri'

module AiMedication
  class SourcePageClient
    TIMEOUT_SECONDS = 5
    MAX_TEXT_LENGTH = 30_000
    NETWORK_ERRORS = [
      Net::OpenTimeout,
      Net::ReadTimeout,
      Errno::ECONNREFUSED,
      Errno::ECONNRESET,
      EOFError,
      SocketError,
      OpenSSL::SSL::SSLError
    ].freeze

    def fetch(url)
      uri = URI.parse(url)
      response = perform_request(uri)
      raise "Source returned #{response.code}" unless response.is_a?(Net::HTTPSuccess)

      body = response.body
      SourcePage.new(
        url: url,
        title: title_from(body),
        text: text_from(body)
      )
    rescue URI::InvalidURIError, *NETWORK_ERRORS => e
      raise "Source fetch failed: #{e.message}"
    end

    private

    def perform_request(uri)
      http = Net::HTTP.new(uri.host, uri.port)
      http.use_ssl = uri.scheme == 'https'
      http.open_timeout = TIMEOUT_SECONDS
      http.read_timeout = TIMEOUT_SECONDS
      http.request(Net::HTTP::Get.new(uri))
    end

    def title_from(html)
      html.to_s[%r{<title[^>]*>(.*?)</title>}im, 1].to_s.squish
    end

    def text_from(html)
      ActionController::Base.helpers.strip_tags(html.to_s).squish.first(MAX_TEXT_LENGTH)
    end
  end
end
