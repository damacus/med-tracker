# frozen_string_literal: true

module Api
  module V1
    class DataExportsController < BaseController
      before_action :no_store

      def show
        render json: { data: export_service.call }
      rescue DataExports::ProfileExportService::Error, PortableData::Encryptor::Error => e
        render_unprocessable(e.message)
      end

      private

      def export_service
        DataExports::ProfileExportService.new(
          household: current_household,
          membership: current_membership,
          mode: params.expect(:mode),
          passphrase: request.headers['X-MedTracker-Portable-Passphrase'].presence,
          request: request
        )
      end
    end
  end
end
