# frozen_string_literal: true

module Api
  module V1
    class MedicationsController < BaseController
      def index
        authorize Medication
        render_collection(policy_scope(Medication), serializer: MedicationSerializer, includes: :location)
      end

      def show
        medication = find_api_record(policy_scope(Medication).includes(:location), params.expect(:id))
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

        record_api_change(medication, action: 'create')
        render_resource(medication.reload, serializer: MedicationSerializer, status: :created)
      end

      def update
        medication = find_api_record(policy_scope(Medication).includes(:location), params.expect(:id))
        authorize medication
        return unless fresh_api_record?(medication)

        medication.paper_trail_event = 'api_update'

        return render_validation_errors(medication) unless medication.update(medication_params)

        record_api_change(medication, action: 'update')
        render_resource(medication.reload, serializer: MedicationSerializer)
      end

      def adjust_inventory
        medication = find_api_record(policy_scope(Medication).includes(:location), params.expect(:id))
        authorize medication, :update?
        result = AdjustMedicationInventoryService.new.call(
          medication: medication,
          new_quantity: params.dig(:adjustment, :new_quantity),
          reason: params.dig(:adjustment, :reason)
        )
        return render_unprocessable(result.error.to_s) unless result.success?

        record_api_change(medication.reload, action: 'update')
        render_resource(medication, serializer: MedicationSerializer)
      end

      def mark_as_ordered
        update_reorder_status(:ordered)
      end

      def mark_as_received
        update_reorder_status(:received)
      end

      private

      def update_reorder_status(status)
        medication = find_api_record(policy_scope(Medication).includes(:location), params.expect(:id))
        authorize medication
        MedicationReorderStatusService.new.call(
          medication: medication,
          status: status,
          order_details: order_details_params
        )
        record_api_change(medication.reload, action: 'update')
        render_resource(medication, serializer: MedicationSerializer)
      end

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

      def order_details_params
        params.fetch(:order_details, ActionController::Parameters.new)
              .permit(:supplier, :quantity, :expected_arrival_on)
              .to_h
              .symbolize_keys
      end
    end
  end
end
