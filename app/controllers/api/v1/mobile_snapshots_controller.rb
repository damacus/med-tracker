# frozen_string_literal: true

module Api
  module V1
    class MobileSnapshotsController < BaseController
      def show
        render json: { data: exporter.payload }
      end

      private

      def exporter
        PortableData::Exporter.new(
          household: current_household,
          membership: current_membership,
          passphrase: nil,
          request: request
        )
      end
    end
  end
end
