# frozen_string_literal: true

require 'net/http'
require 'uri'

module DemoReset
  class HealthChecker
    TIMEOUT_SECONDS = 5

    def initialize(application_url: ENV.fetch('APP_URL', nil))
      @application_url = application_url
    end

    def call
      uri = URI.join(application_url.to_s, '/up')
      http = Net::HTTP.new(uri.host, uri.port)
      http.use_ssl = uri.scheme == 'https'
      http.open_timeout = TIMEOUT_SECONDS
      http.read_timeout = TIMEOUT_SECONDS
      http.request(Net::HTTP::Get.new(uri)).is_a?(Net::HTTPSuccess)
    rescue URI::Error, SocketError, SystemCallError, Timeout::Error
      false
    end

    private

    attr_reader :application_url
  end
end
