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
        medication.created_by_membership_id = current_membership.id
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
        ).tap { |permitted| constrain_medication_location!(permitted) }
      end

      def constrain_medication_location!(permitted)
        location_id = permitted[:location_id].presence
        return if location_id.blank?

        permitted[:location_id] = policy_scope(Location).find(location_id).id
      end
    end
  end
end
