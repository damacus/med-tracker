# frozen_string_literal: true

module Api
  module V1
    class PortableExportsController < BaseController
      def show
        return render_unprocessable('Portable passphrase header is required') if portable_passphrase.blank?

        render json: { data: exporter.call }
      rescue PortableData::Encryptor::Error => e
        render_unprocessable(e.message)
      end

      private

      def exporter
        PortableData::Exporter.new(
          household: current_household,
          membership: current_membership,
          passphrase: portable_passphrase,
          request: request
        )
      end

      def portable_passphrase
        request.headers['X-MedTracker-Portable-Passphrase'].presence
      end
    end
  end
end
