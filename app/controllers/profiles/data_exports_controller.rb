# frozen_string_literal: true

module Profiles
  class DataExportsController < ApplicationController
    before_action :require_authentication

    def show
      result = export_service.call
      return send_zip(result) if params.expect(:mode) == 'backup_zip'

      render json: result
    rescue DataExports::ProfileExportService::Error, PortableData::Encryptor::Error => e
      redirect_to profile_path, alert: e.message
    end

    private

    def export_service
      DataExports::ProfileExportService.new(
        household: current_household,
        membership: current_membership,
        mode: params.expect(:mode),
        request: request
      )
    end

    def send_zip(result)
      send_data Base64.strict_decode64(result.fetch(:base64)),
                filename: result.fetch(:filename),
                type: result.fetch(:content_type)
    end
  end
end
