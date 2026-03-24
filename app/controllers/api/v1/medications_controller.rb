# frozen_string_literal: true

module Api
  module V1
    class MedicationsController < BaseController
      def index
        authorize Medication
        render_collection(policy_scope(Medication), serializer: MedicationSerializer, includes: :location)
      end

      def show
        medication = policy_scope(Medication).includes(:location).find(params[:id])
        authorize medication

        render_resource(medication, serializer: MedicationSerializer)
      end
    end
  end
end
