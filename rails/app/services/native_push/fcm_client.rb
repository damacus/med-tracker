# frozen_string_literal: true

module NativePush
  class FcmClient
    UNREGISTERED_ERROR_CODES = %w[UNREGISTERED INVALID_ARGUMENT].freeze

    class << self
      def configured?
        project_id.present? && bearer_token.present?
      end

      def project_id
        ENV['FCM_PROJECT_ID'].presence || Rails.application.credentials.dig(:fcm, :project_id)
      end

      def bearer_token
        ENV['FCM_BEARER_TOKEN'].presence || Rails.application.credentials.dig(:fcm, :bearer_token)
      end
    end

    def initialize(connection: nil)
      @connection = connection || Faraday.new(url: 'https://fcm.googleapis.com')
    end

    def deliver(token, title:, body:, path: '/')
      response = connection.post(messages_path, payload(token: token, title: title, body: body, path: path).to_json,
                                 headers)
      result_for(response)
    rescue StandardError => e
      DeliveryResult.failed(provider_status: nil, provider_error: e.class.name)
    end

    private

    attr_reader :connection

    def messages_path
      "/v1/projects/#{self.class.project_id}/messages:send"
    end

    def headers
      {
        'authorization' => "Bearer #{self.class.bearer_token}",
        'content-type' => 'application/json'
      }
    end

    def payload(token:, title:, body:, path:)
      {
        message: {
          token: token.device_token,
          notification: {
            title: title,
            body: body
          },
          data: {
            path: path
          }
        }
      }
    end

    def result_for(response)
      return DeliveryResult.delivered(provider_status: response.status) if response.status == 200

      error_code = fcm_error_code(response)
      return DeliveryResult.unregistered(provider_status: response.status, provider_error: error_code) if
        UNREGISTERED_ERROR_CODES.include?(error_code)

      DeliveryResult.failed(provider_status: response.status, provider_error: error_code)
    end

    def fcm_error_code(response)
      body = JSON.parse(response.body.to_s)
      details = Array(body.dig('error', 'details'))
      fcm_detail = details.find { |detail| detail['@type'].to_s.include?('google.firebase.fcm.v1.FcmError') }
      fcm_detail&.fetch('errorCode', nil) || body.dig('error', 'status')
    rescue JSON::ParserError
      nil
    end
  end
end
