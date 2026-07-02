# frozen_string_literal: true

require 'jwt'

module NativePush
  class ApnsClient
    class << self
      def configured?
        bundle_id.present? && team_id.present? && key_id.present? && private_key.present?
      end

      def bundle_id
        ENV['APNS_BUNDLE_ID'].presence || Rails.application.credentials.dig(:apns, :bundle_id)
      end

      def team_id
        ENV['APNS_TEAM_ID'].presence || Rails.application.credentials.dig(:apns, :team_id)
      end

      def key_id
        ENV['APNS_KEY_ID'].presence || Rails.application.credentials.dig(:apns, :key_id)
      end

      def private_key
        key = ENV['APNS_PRIVATE_KEY'].presence || Rails.application.credentials.dig(:apns, :private_key)
        key&.gsub('\n', "\n")
      end
    end

    def initialize(connection: nil)
      @connection = connection || Faraday.new(url: apns_host)
    end

    def deliver(token, title:, body:, path: '/')
      response = connection.post(device_path(token), payload(title: title, body: body, path: path).to_json, headers)
      result_for(response)
    rescue StandardError => e
      DeliveryResult.failed(provider_status: nil, provider_error: e.class.name)
    end

    private

    attr_reader :connection

    def device_path(token)
      "/3/device/#{token.device_token}"
    end

    def headers
      {
        'authorization' => "bearer #{authentication_token}",
        'apns-topic' => self.class.bundle_id,
        'apns-push-type' => 'alert',
        'apns-priority' => '10',
        'content-type' => 'application/json'
      }
    end

    def apns_host
      ENV['APNS_HOST'].presence || Rails.application.credentials.dig(:apns, :host) || default_apns_host
    end

    def default_apns_host
      sandbox = ActiveModel::Type::Boolean.new.cast(
        ENV.fetch('APNS_SANDBOX', Rails.application.credentials.dig(:apns, :sandbox))
      )
      sandbox ? 'https://api.sandbox.push.apple.com' : 'https://api.push.apple.com'
    end

    def authentication_token
      JWT.encode(
        { iss: self.class.team_id, iat: Time.current.to_i },
        OpenSSL::PKey::EC.new(self.class.private_key),
        'ES256',
        kid: self.class.key_id
      )
    end

    def payload(title:, body:, path:)
      {
        aps: {
          alert: {
            title: title,
            body: body
          },
          sound: 'default'
        },
        path: path
      }
    end

    def result_for(response)
      return DeliveryResult.delivered(provider_status: response.status) if response.status == 200
      return DeliveryResult.unregistered(provider_status: response.status, provider_error: error_reason(response)) if
        response.status == 410 || error_reason(response) == 'BadDeviceToken'

      DeliveryResult.failed(provider_status: response.status, provider_error: error_reason(response))
    end

    def error_reason(response)
      JSON.parse(response.body.to_s).fetch('reason', nil)
    rescue JSON::ParserError
      nil
    end
  end
end
