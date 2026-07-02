# frozen_string_literal: true

module Api
  module V1
    class MedicationsController < BaseController
      def index
        authorize Medication
        render_collection(policy_scope(Medication), serializer: MedicationSerializer, includes: :location)
      end

      def show
        medication = policy_scope(Medication).includes(:location).find(params.expect(:id))
        authorize medication

        render_resource(medication, serializer: MedicationSerializer)
      end

      def create
        medication = Medication.new(medication_params)
        medication.household = current_household
        medication.paper_trail_event = 'api_create'
        authorize medication

        return render_validation_errors(medication) unless medication.save

        render_resource(medication.reload, serializer: MedicationSerializer, status: :created)
      end

      def update
        medication = policy_scope(Medication).includes(:location).find(params.expect(:id))
        authorize medication
        medication.paper_trail_event = 'api_update'

        return render_validation_errors(medication) unless medication.update(medication_params)

        render_resource(medication.reload, serializer: MedicationSerializer)
      end

      private

      def medication_params
        params.expect(
          medication: %i[
            name
            friendly_name
            barcode
            dmd_code
            dmd_system
            dmd_concept_class
            category
            description
            dose_amount
            dose_unit
            current_supply
            reorder_threshold
            warnings
            location_id
            default_schedule_type
          ]
        )
      end
    end
  end
end
