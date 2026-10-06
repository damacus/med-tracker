# frozen_string_literal: true

module Api
  module V1
    class PortableExportsController < BaseController
      def show
        return render_unprocessable('Portable passphrase header is required') if portable_passphrase.blank?

        return render_unprocessable('Unsupported portable data version') unless portable_version_valid?

        render json: { data: exporter.call(version: params[:version].to_s == '2' ? 2 : 1,
                                           format: params.fetch(:portable_format, PortableData::Exporter::FORMAT)) }
      rescue PortableData::Encryptor::Error, PortableData::Exporter::Error => e
        render_unprocessable(e.message)
      end

      private

      def portable_version_valid?
        params[:version].nil? || params[:version].to_s.in?(%w[1 2])
      end

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
