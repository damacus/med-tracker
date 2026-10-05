# frozen_string_literal: true

module Api
  module V1
    module ErrorRendering
      private

      def render_api_error(code:, message:, status:, errors: nil)
        payload = {
          code: code,
          message: message,
          request_id: request.request_id
        }
        payload[:errors] = errors if errors.present?

        render json: { error: payload }, status: status
      end
    end
  end
end
