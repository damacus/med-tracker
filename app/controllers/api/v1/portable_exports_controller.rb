# frozen_string_literal: true

module Api
  module V1
    class PortableExportsController < BaseController
      def show
        render json: { data: exporter.call }
      rescue PortableData::Encryptor::Error => e
        render_unprocessable(e.message)
      end

      private

      def exporter
        PortableData::Exporter.new(
          household: current_household,
          membership: current_membership,
          passphrase: params.require(:passphrase),
          request: request
        )
      end
    end
  end
end
