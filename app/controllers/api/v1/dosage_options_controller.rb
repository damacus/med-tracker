# frozen_string_literal: true

module Api
  module V1
    class DosageOptionsController < BaseController
      def index
        authorize MedicationDosageOption
        render_collection(policy_scope(MedicationDosageOption), serializer: DosageOptionSerializer, includes: :medication)
      end

      def show
        dosage_option = find_api_record(policy_scope(MedicationDosageOption).includes(:medication), params.expect(:id))
        authorize dosage_option

        render_resource(dosage_option, serializer: DosageOptionSerializer)
      end

      def create
        dosage_option = MedicationDosageOption.new(dosage_option_params)
        dosage_option.household = current_household
        authorize dosage_option

        return render_validation_errors(dosage_option) unless dosage_option.save

        record_api_change(dosage_option, action: 'create')
        render_resource(dosage_option.reload, serializer: DosageOptionSerializer, status: :created)
      end

      def update
        dosage_option = find_api_record(policy_scope(MedicationDosageOption).includes(:medication), params.expect(:id))
        authorize dosage_option
        return unless fresh_api_record?(dosage_option)

        return render_validation_errors(dosage_option) unless dosage_option.update(dosage_option_update_params)

        record_api_change(dosage_option, action: 'update')
        render_resource(dosage_option.reload, serializer: DosageOptionSerializer)
      end

      private

      def dosage_option_params
        params.expect(
          dosage_option: %i[
            medication_id
            amount
            unit
            frequency
            description
            default_for_adults
            default_for_children
            default_max_daily_doses
            default_min_hours_between_doses
            default_dose_cycle
            current_supply
            reorder_threshold
          ]
        ).tap do |attributes|
          if attributes[:medication_id].present?
            attributes[:medication_id] = api_record_id(policy_scope(Medication), attributes[:medication_id])
          end
        end
      end

      def dosage_option_update_params
        dosage_option_params.except(:medication_id)
      end
    end
  end
end
