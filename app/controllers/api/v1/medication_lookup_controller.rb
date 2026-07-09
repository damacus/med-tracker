# frozen_string_literal: true

module Api
  module V1
    class MedicationLookupController < BaseController
      def show
        authorize Medication, :finder?
        response = MedicationFinderSearchResponder.new(medication_scope: policy_scope(Medication)).call(
          query: params[:q],
          form: params[:form],
          permissions: {
            can_create: policy(Medication).create?,
            can_update: false
          }
        )

        render json: response.body, status: response.status
      end
    end
  end
end
