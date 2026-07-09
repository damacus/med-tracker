# frozen_string_literal: true

module Api
  module V1
    class DosageOptionSerializer
      def initialize(dosage_option)
        @dosage_option = dosage_option
      end

      def as_json(*)
        identity_data.merge(dose_data).merge(default_data).merge(inventory_data)
      end

      private

      attr_reader :dosage_option

      def identity_data
        {
          id: dosage_option.id,
          portable_id: dosage_option.portable_id,
          medication_id: dosage_option.medication_id,
          medication_portable_id: dosage_option.medication&.portable_id
        }
      end

      def dose_data
        {
          amount: dosage_option.amount,
          unit: dosage_option.unit,
          frequency: dosage_option.frequency,
          description: dosage_option.description
        }
      end

      def default_data
        {
          default_for_adults: dosage_option.default_for_adults,
          default_for_children: dosage_option.default_for_children,
          default_max_daily_doses: dosage_option.default_max_daily_doses,
          default_min_hours_between_doses: dosage_option.default_min_hours_between_doses,
          default_dose_cycle: dosage_option.default_dose_cycle
        }
      end

      def inventory_data
        {
          current_supply: dosage_option.current_supply,
          reorder_threshold: dosage_option.reorder_threshold,
          updated_at: dosage_option.updated_at.iso8601
        }
      end
    end
  end
end
