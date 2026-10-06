# frozen_string_literal: true

module Api
  module V1
    class MedicationLookupController < BaseController
      include DecimalSerialization

      def show
        authorize Medication, :finder?
        response = MedicationFinderSearchResponder.new(medication_scope: policy_scope(Medication)).call(
          query: params[:q],
          form: params[:form],
          strength: params[:strength],
          permissions: {
            can_create: policy(Medication).create?,
            can_update: false
          }
        )

        render json: lookup_response_body(response.body), status: response.status
      end

      private

      def lookup_response_body(body)
        return body unless body[:results]

        body.merge(results: body[:results].map { |result| serialize_result(result) })
      end

      def serialize_result(result)
        return result unless result.key?(:package_quantity)

        result.merge(package_quantity: decimal_as_json(result[:package_quantity]))
      end
    end
  end
end
